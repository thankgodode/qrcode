use std::{fs::File, io::Read};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose};
use image::Luma;
use qrcode::QrCode;
use tokio::{io::AsyncReadExt};
use uuid::Uuid;

use qrcode_gen::{
    Db,
    state::{Response, UserInfo},
};

const MAX_SIGNATURE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    db: Db,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let db = Db::new().await?;
    db.create_table().await?;
    println!("Table created successfully");

    let state = AppState { db };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await?;

    let app = Router::new()
        .route("/", get(|| async { "hello world" }))
        .route("/create_user", post(create_user))
        .route("/{id}", get(get_user))
        .with_state(state);

    println!("Server running on http://192.168.43.38:5000");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_user(
    State(pool): State<AppState>,
    Json(user_info): Json<UserInfo>,
) -> impl IntoResponse {
    let db = pool.db;
    let id = Uuid::new_v4();

    let qr_code = match generate_qrcode(id) {
        Ok(qr_code) => qr_code,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Response {
                    created: "Error generating QR code".into(),
                }),
            );
        }
    };

    let signature = match parse_signature(&user_info.signature) {
        Ok(signature) => signature,
        Err(status) => {
            return (
                status,
                Json(Response {
                    created: "Error parsing signature".into(),
                }),
            );
        }
    };

    match db.insert(id, user_info, qr_code, signature).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(Response {
                created: "User created".into(),
            }),
        ),
        Err(error) => {
            eprintln!("failed to insert employee {id}: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Response {
                    created: "Error creating user".into(),
                }),
            )
        }
    }
}

async fn get_user(
    State(pool): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let db = pool.db;
    let response = db
        .get_employee(id)
        .await
        .map_err(|error| {
            eprintln!("failed to fetch employee {id}: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let signature_b64 = general_purpose::STANDARD.encode(&response.signature);
    let qr_b64 = general_purpose::STANDARD.encode(&response.qr_code);

    let first_name = escape_html(&response.first_name);
    let last_name = escape_html(&response.last_name);
    let contact = escape_html(&response.contact);

    let html = format!(
        r#"
        <head>
            <title>User Info</title>
            </head>
            <body>
                <h1>User Info</h1>
                <p>ID: {id}</p>
                <p>First Name: {first_name}</p>
                <p>Last Name: {last_name}</p>
                <p>Signature: <img src="data:image/png;base64,{signature_b64}"/></p>
                <p>Contact: {contact}</p>
                <img src="data:image/png;base64,{qr_b64}" alt="QR Code">
            </body>
        </html>
    "#,
    );

    Ok(Html(html))
}

fn generate_qrcode(id: Uuid) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| format!("http://192.168.43.38:5000/{id}"));

    let code = QrCode::new(base_url)?;
    let image = code.render::<Luma<u8>>().build();

    image.save(format!("./src/{}.png", id))?;

    Ok(image.to_vec())
}

fn parse_signature(encoded: &str) -> Result<Vec<u8>, StatusCode> {
    let mut buf = vec![];
    let mut reader = File::open(encoded)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let _= reader.read_to_end(&mut buf).map_err(|_| StatusCode::BAD_REQUEST);


    // let signature = general_purpose::STANDARD
    //     .decode(&buf)
    //     .map_err(|_| StatusCode::BAD_REQUEST)?;

    // if signature.len() > MAX_SIGNATURE_BYTES {
    //     return Err(StatusCode::PAYLOAD_TOO_LARGE);
    // }

    Ok(buf)
}

fn escape_html(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());

    for character in input.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            character => escaped.push(character),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html_metacharacters() {
        assert_eq!(
            escape_html(r#"<script>"x" & 'y'</script>"#),
            "&lt;script&gt;&quot;x&quot; &amp; &#x27;y&#x27;&lt;/script&gt;"
        );
    }

    #[test]
    fn decodes_base64_signature() {
        let encoded = general_purpose::STANDARD.encode([0x89, b'P', b'N', b'G']);

        assert_eq!(parse_signature(&encoded), Ok(vec![0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn rejects_invalid_base64_signature() {
        assert_eq!(parse_signature("not base64!"), Err(StatusCode::BAD_REQUEST));
    }
}
