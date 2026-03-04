use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginResponse {
    pub access_token: Option<String>,
    pub message: Option<String>,
}
