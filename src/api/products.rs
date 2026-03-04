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
    let variations: Vec<NewVariationPayload> = product
        .variations
        .iter()
        .enumerate()
        .map(|(i, v)| NewVariationPayload {
            price: v.price,
            dimensions: v.dimensions.clone(),
            packaging: v.packaging.clone(),
            standard: v.standard.clone(),
            // product-level description goes to the first variation
            description: if i == 0 { product.description.clone().or_else(|| v.description.clone()) }
                         else { v.description.clone() },
        })
        .collect();

    let payload = NewProductPayload {
        name: product.name.clone(),
        category: product.category.clone(),
        variations,
    };

    let data_json = serde_json::to_string(&payload)?;
    let mut form = reqwest::multipart::Form::new().text("data", data_json);

    if let Some(img_path) = &product.image_path {
        let img_data = tokio::fs::read(img_path).await?;
        let filename = std::path::Path::new(img_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.jpg")
            .to_string();
        let mime = if filename.ends_with(".png") {
            "image/png"
        } else if filename.ends_with(".webp") {
            "image/webp"
        } else {
            "image/jpeg"
        };
        let part = reqwest::multipart::Part::bytes(img_data)
            .file_name(filename)
            .mime_str(mime)?;
        form = form.part("image", part);
    }

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub price: f64,
}

pub async fn update_variation(
    client: &Client,
    base_url: &str,
    token: &str,
    _product_id: i64,
    variation_id: i64,
    update: VariationUpdate,
) -> Result<(), ApiError> {
    let res = client
        .put(format!("{base_url}/product/variation/{variation_id}/"))
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
        .post(format!("{base_url}/product/{product_id}/variations"))
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

pub async fn download_catalog_pdf(
    client: &Client,
    base_url: &str,
    token: &str,
) -> Result<Vec<u8>, ApiError> {
    let res = client
        .get(format!("{base_url}/catalog/pdf"))
        .bearer_auth(token)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("download_catalog_pdf failed {status}: {body}").into());
    }

    eprintln!("[api/products] catalog PDF downloaded");
    Ok(res.bytes().await?.to_vec())
}

pub async fn update_product_image(
    client: &Client,
    base_url: &str,
    token: &str,
    product_id: i64,
    image_path: &str,
) -> Result<ProductData, ApiError> {
    let img_data = tokio::fs::read(image_path).await?;
    let mime = if image_path.ends_with(".png") {
        "image/png"
    } else if image_path.ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg"
    };

    let res = client
        .put(format!("{base_url}/product/{product_id}"))
        .bearer_auth(token)
        .header("Content-Type", mime)
        .body(img_data)
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;
    eprintln!("[api/products] update_product_image {product_id} status: {status}");

    if !status.is_success() {
        return Err(format!("update_product_image failed {status}: {body}").into());
    }

    Ok(serde_json::from_str(&body)?)
}
