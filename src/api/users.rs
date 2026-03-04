use reqwest::Client;
use serde::{Deserialize, Serialize};

type ApiError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Deserialize, Debug, Clone)]
pub struct UserData {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub email: String,
    pub is_active: bool,
    #[serde(default)]
    pub is_superuser: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
    pub password: String,
    pub is_active: bool,
    pub is_superuser: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct UpdateUser {
    pub name: String,
    pub email: String,
    pub is_active: bool,
    pub is_superuser: bool,
}

pub async fn fetch_all(
    client: &Client,
    base_url: &str,
    token: &str,
) -> Result<Vec<UserData>, ApiError> {
    let res = client
        .get(format!("{base_url}/user/"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    eprintln!("[api/users] fetch_all status: {status}");

    Ok(serde_json::from_str(&body)?)
}

pub async fn create(
    client: &Client,
    base_url: &str,
    token: &str,
    user: CreateUser,
) -> Result<(), ApiError> {
    let res = client
        .post(format!("{base_url}/user/"))
        .bearer_auth(token)
        .json(&user)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("create_user failed {status}: {body}").into());
    }
    eprintln!("[api/users] user created");
    Ok(())
}

pub async fn update(
    client: &Client,
    base_url: &str,
    token: &str,
    user_id: i64,
    user: UpdateUser,
) -> Result<(), ApiError> {
    let res = client
        .put(format!("{base_url}/user/{user_id}/"))
        .bearer_auth(token)
        .json(&user)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("update_user failed {status}: {body}").into());
    }
    eprintln!("[api/users] user {user_id} updated");
    Ok(())
}

pub async fn delete(
    client: &Client,
    base_url: &str,
    token: &str,
    user_id: i64,
) -> Result<(), ApiError> {
    let res = client
        .delete(format!("{base_url}/user/{user_id}/"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("delete_user failed {status}: {body}").into());
    }
    eprintln!("[api/users] user {user_id} deleted");
    Ok(())
}
