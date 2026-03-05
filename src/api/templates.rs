use reqwest::Client;
use serde::Deserialize;

type ApiError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateData {
    pub id: i64,
    pub name: String,
}

pub async fn fetch_all(
    client: &Client,
    base_url: &str,
    token: &str,
) -> Result<Vec<TemplateData>, ApiError> {
    let res = client
        .get(format!("{base_url}/template"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    eprintln!("[api/templates] fetch_all status: {status}");
    Ok(serde_json::from_str(&body)?)
}
