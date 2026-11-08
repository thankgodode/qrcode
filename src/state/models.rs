use serde::{Deserialize, Serialize};

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Employee {
    pub id: uuid::Uuid,
    pub first_name: String,
    pub last_name: String,
    pub contact: String,
    pub signature: Vec<u8>,
    pub qr_code: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub created: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UserInfo {
    pub first_name: String,
    pub last_name: String,
    pub contact: String,
    /// Base64-encoded PNG containing the user's signature.
    pub signature: String,
}
