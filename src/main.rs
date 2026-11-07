mod state;

use axum::{
    Json, Router, extract::{OriginalUri, Path, Request, State}, http::{HeaderMap, StatusCode, header}, response::{Html, IntoResponse}, routing::{get, post},
};

use base64::{Engine, engine::{self, GeneralPurpose, general_purpose}};
use serde::{Deserialize, Serialize};
use qrcode::QrCode;
use image::Luma;
use sqlx::pool;
use tokio::{fs::File, io::AsyncReadExt};
use uuid::Uuid;

use qrcode_gen::{Db, state::state::{Employee, UserInfo}};

#[derive(Clone)]
struct AppState {
    db: Db,
}

#[tokio::main]
async fn main()-> Result<(), Box<dyn std::error::Error>>{
    dotenvy::dotenv().ok();

    let db = Db::new().await.unwrap();
    
    let _  = match db.create_table().await{
        Ok(_) => println!("Table created successfully"),
        Err(e) => panic!("Error creating table: {}", e)
    };

    let state = AppState { db };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await?;

    let app = Router::new()
        .route("/", get(|| async{"hello world"}))
        .route("/create_user", post(create_user))
        .route("/{id}", get(get_user))
        .with_state(state);

        
    println!("Server running on http://192.168.43.38:5000");

    axum::serve(listener, app).await?;


    Ok(())

}


async fn create_user(State(pool):State<AppState>, Json(user_info): Json<UserInfo>)-> impl IntoResponse{
    let db = pool.db;
    let id = Uuid::new_v4();

    let qr_code = if let Ok(qrcode) = generate_qrcode(id){qrcode} else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(state::state::Response { created: "Error generating QR code".into() })
        );
    };
    let signature = if let Ok(signature) = parse_signature(user_info.signature.clone()).await {
        signature
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(state::state::Response { created: "Error parsing signature".into() })
        );
    };
    
    if let Ok(()) = db.insert(id, user_info, qr_code, signature).await {
        return (
            StatusCode::CREATED,
            Json(state::state::Response { created: "User created".into() })
        )
    } else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(state::state::Response { created: "Error creating user".into() })
        );
    };

    
}

async fn get_user(
    State(pool):State<AppState>,
    Path(id): Path<Uuid>,
)-> Html<String>{
    let db = pool.db;
    let response = db.get_employee(id).await.unwrap();
    let signature_b64 = general_purpose::STANDARD.encode(&response.signature);

    let signature_b64 = format!("data:image/png;base64,{}",signature_b64);

    let html = format!(
    r#"
        <head>
            <title>User Info</title>
            </head>
            <body>
                <h1>User Info</h1>
                <p>ID: {}</p>
                <p>First Name: {}</p>
                <p>Last Name: {}</p>
                <p>Signature: <img src={:?}/></p>
                <p>Contact: {}</p>
                <img src="data:image/png;base64" alt="QR Code">
            </body>
        </html>
    "#, 
        response.id,
        response.first_name,
        response.last_name,
        signature_b64,
        response.contact
    );

    println!("SIGN: {:?}",html);

    Html(html)
}

fn generate_qrcode(id: Uuid)-> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| format!("http://192.168.43.38:5000/{}",id).into());
    
    let code = QrCode::new(base_url)?;
    let image = code.render::<Luma<u8>>().build();

    image.save(format!("./src/{}.png", id))?;

    Ok(image.to_vec())
}

async fn parse_signature(signature: String)-> Result<Vec<u8>, Box<dyn std::error::Error>>{
    let mut signature_bytes = vec![];
    let mut file = File::open(signature).await?;
    file.read_to_end(&mut signature_bytes).await?;

    Ok(signature_bytes)
}