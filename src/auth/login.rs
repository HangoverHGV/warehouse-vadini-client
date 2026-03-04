use crate::models::user::{LoginPayload, LoginResponse};
use reqwest::Client;

pub async fn login(
    client: &Client,
    url: &str,
    username: &str,
    password: &str,
) -> Result<LoginResponse, reqwest::Error> {
    let payload = LoginPayload {
        username: username.to_string(),
        password: password.to_string(),
    };

    let res = client
        .post(url)
        .form(&payload)
        .send()
        .await?
        .json::<LoginResponse>()
        .await?;

    Ok(res)
}

pub async fn get_protected(client: &Client, url: &str) -> Result<String, reqwest::Error> {
    let body = client.get(url).send().await?.text().await?;

    Ok(body)
}
