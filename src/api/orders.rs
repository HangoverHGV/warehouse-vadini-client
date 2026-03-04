use reqwest::Client;
use serde::{Deserialize, Serialize};

type ApiError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Debug, Clone)]
pub struct OrderItemCreate {
    pub variation_id: i64,
    pub quantity: i64,
    pub discount: f64,
}

#[derive(Serialize, Debug, Clone)]
pub struct OrderCreate {
    pub client_name: String,
    pub address_invoice: String,
    pub address_delivery: String,
    pub discount: f64,
    pub items: Vec<OrderItemCreate>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct VariationInItem {
    pub id: i64,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub dimensions: Option<String>,
    #[serde(default)]
    pub packaging: Option<String>,
    #[serde(default)]
    pub standard: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct OrderItemData {
    pub variation_id: i64,
    pub quantity: i64,
    #[serde(default)]
    pub discount: Option<f64>,
    #[serde(default)]
    pub variation: Option<VariationInItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrderData {
    pub id: i64,
    pub client_name: String,
    #[serde(default)]
    pub address_invoice: Option<String>,
    #[serde(default)]
    pub address_delivery: Option<String>,
    pub total: f64,
    #[serde(default)]
    pub discount: Option<f64>,
    pub created_at: String,
    #[serde(default)]
    pub items: Vec<OrderItemData>,
}

pub async fn fetch_all(
    client: &Client,
    base_url: &str,
    token: &str,
) -> Result<Vec<OrderData>, ApiError> {
    let res = client
        .get(format!("{base_url}/order/"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    eprintln!("[api/orders] fetch_all status: {status}");

    Ok(serde_json::from_str(&body)?)
}

pub async fn fetch_one(
    client: &Client,
    base_url: &str,
    token: &str,
    id: i64,
) -> Result<OrderData, ApiError> {
    let res = client
        .get(format!("{base_url}/order/{id}"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    eprintln!("[api/orders] fetch_one {id} status: {status}");

    Ok(serde_json::from_str(&body)?)
}

pub async fn create(
    client: &Client,
    base_url: &str,
    token: &str,
    order: OrderCreate,
) -> Result<(), ApiError> {
    let res = client
        .post(format!("{base_url}/order/"))
        .bearer_auth(token)
        .json(&order)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Order failed {status}: {body}").into());
    }

    eprintln!("[api/orders] order created, status: {status}");
    Ok(())
}
