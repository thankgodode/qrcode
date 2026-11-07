pub mod state;

use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};
use image::Luma;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::{fs::File, io::AsyncReadExt};
use uuid::Uuid;

use state::state::{Employee, UserInfo};

#[derive(Debug, Clone)]
pub struct Db {
    pub pool: PgPool,
}

impl Db {
    pub async fn new() -> Result<Self, sqlx::Error> {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://thankgod:postgr@localhost/qrcode".into());

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        Ok(Db { pool })
    }

    pub async fn create_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS employees (
                id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
                first_name TEXT NOT NULL,
                last_name TEXT NOT NULL,
                contact VARCHAR(11) NOT NULL,
                signature BYTEA NOT NULL,
                qr_code BYTEA NOT NULL
            )"#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert(&self, id: Uuid, user_info: UserInfo, qr_code: Vec<u8>, signature: Vec<u8>) -> Result<(), sqlx::Error> {

        sqlx::query!(
            r#"INSERT INTO employees (id, first_name, last_name, contact, signature, qr_code) VALUES ($1, $2, $3, $4, $5, $6)"#,
            id,
            user_info.first_name,
            user_info.last_name,
            user_info.contact,
            signature,
            qr_code
        )
        .execute(&self.pool)
        .await
        .unwrap();

        Ok(())
    }

    pub async fn get_employee(&self, id: Uuid) -> Result<Employee, StatusCode> {
        let pool = Db::new().await.unwrap();

        let employee = sqlx::query_as::<_, Employee>(
            "SELECT * FROM employees WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&pool.pool)
        .await
        .map_err(|e| {
            println!("Error: {:?}", e.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

        Ok(employee)
    }

    pub async fn clear_db(&self){
        
    }
}

