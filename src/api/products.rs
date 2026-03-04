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
