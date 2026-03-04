use reqwest::Client;

use crate::models::product::{NewProduct, NewProductPayload, NewVariationPayload, ProductData};

type ApiError = Box<dyn std::error::Error + Send + Sync>;

pub async fn fetch_all(
    client: &Client,
    base_url: &str,
    token: &str,
) -> Result<Vec<ProductData>, ApiError> {
    let res = client
        .get(format!("{base_url}/product/"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    eprintln!("[api/products] fetch_all status: {status}");

    Ok(serde_json::from_str(&body)?)
}

pub async fn fetch_one(
    client: &Client,
    base_url: &str,
    token: &str,
    id: i64,
) -> Result<ProductData, ApiError> {
    let res = client
        .get(format!("{base_url}/product/{id}"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    eprintln!("[api/products] fetch_one {id} status: {status}");

    Ok(serde_json::from_str(&body)?)
}

pub async fn create(
    client: &Client,
    base_url: &str,
    token: &str,
    product: &NewProduct,
) -> Result<ProductData, ApiError> {
    let payload = NewProductPayload {
        name: product.name.clone(),
        category: product.category.clone(),
        variations: vec![NewVariationPayload {
            price: product.price,
            description: product.description.clone(),
        }],
    };

    let data_json = serde_json::to_string(&payload)?;
    let form = reqwest::multipart::Form::new().text("data", data_json);

    let res = client
        .post(format!("{base_url}/product/"))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    eprintln!("[api/products] create status: {status}");

    if !status.is_success() {
        return Err(format!("Create failed {status}: {body}").into());
    }

    Ok(serde_json::from_str(&body)?)
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct VariationUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packaging: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    pub price: f64,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct NewVariation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packaging: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    pub price: f64,
}

pub async fn update_variation(
    client: &Client,
    base_url: &str,
    token: &str,
    product_id: i64,
    variation_id: i64,
    update: VariationUpdate,
) -> Result<(), ApiError> {
    let res = client
        .put(format!("{base_url}/product/{product_id}/variation/{variation_id}/"))
        .bearer_auth(token)
        .json(&update)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("update_variation failed {status}: {body}").into());
    }
    eprintln!("[api/products] variation {variation_id} updated");
    Ok(())
}

pub async fn delete_variation(
    client: &Client,
    base_url: &str,
    token: &str,
    product_id: i64,
    variation_id: i64,
) -> Result<(), ApiError> {
    let res = client
        .delete(format!("{base_url}/product/{product_id}/variation/{variation_id}/"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("delete_variation failed {status}: {body}").into());
    }
    eprintln!("[api/products] variation {variation_id} deleted");
    Ok(())
}

pub async fn add_variation(
    client: &Client,
    base_url: &str,
    token: &str,
    product_id: i64,
    variation: NewVariation,
) -> Result<(), ApiError> {
    let res = client
        .post(format!("{base_url}/product/{product_id}/variation/"))
        .bearer_auth(token)
        .json(&variation)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("add_variation failed {status}: {body}").into());
    }
    eprintln!("[api/products] variation added to product {product_id}");
    Ok(())
}

pub async fn delete_product(
    client: &Client,
    base_url: &str,
    token: &str,
    product_id: i64,
) -> Result<(), ApiError> {
    let res = client
        .delete(format!("{base_url}/product/{product_id}/"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("delete_product failed {status}: {body}").into());
    }
    eprintln!("[api/products] product {product_id} deleted");
    Ok(())
}
