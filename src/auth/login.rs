use crate::models::user::{LoginPayload, LoginResponse};
use reqwest::Client;

type LoginError = Box<dyn std::error::Error + Send + Sync>;

pub async fn login(
    client: &Client,
    url: &str,
    username: &str,
    password: &str,
) -> Result<LoginResponse, LoginError> {
    let payload = LoginPayload {
        username: username.to_string(),
        password: password.to_string(),
    };

    let response = client.post(url).form(&payload).send().await?;

    let status = response.status();
    let body = response.text().await?;

    eprintln!("[login] status: {status}");
    eprintln!("[login] body:   {body}");

    let res = serde_json::from_str::<LoginResponse>(&body)?;
    Ok(res)
}

/// Validates the stored token against /user/me.
/// Returns the new token if the server rotated it.
/// Returns `Err` only for network errors or a 401 Unauthorized response —
/// the caller should clear the stored token only in those cases.
/// Any other non-success status (500, 503, …) returns `Ok(Some(token))`
/// so the caller keeps the token in config for the next run.
pub async fn check_login(
    client: &Client,
    url: &str,
    token: &str,
) -> Result<Option<String>, LoginError> {
    let response = client.get(url).bearer_auth(token).send().await?;
    let status = response.status();
    let body = response.text().await?;

    eprintln!("[check_login] status: {status}");
    eprintln!("[check_login] body:   {body}");

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(format!("Token rejected: {status}").into());
    }

    if !status.is_success() {
        // Server-side error — not the token's fault; preserve it for next run.
        eprintln!("[check_login] non-fatal server error ({status}), keeping token");
        return Ok(Some(token.to_string()));
    }

    let new_token = serde_json::from_str::<LoginResponse>(&body)
        .ok()
        .and_then(|r| r.access_token);

    Ok(new_token)
}

pub async fn get_protected(client: &Client, url: &str) -> Result<String, reqwest::Error> {
    let body = client.get(url).send().await?.text().await?;

    Ok(body)
}
