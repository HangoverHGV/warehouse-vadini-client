use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Deserialize, Serialize, Debug, Clone, Default)]
pub struct VariationData {
    pub id: i64,
    pub product_id: i64,
    pub dimensions: Option<String>,
    pub packaging: Option<String>,
    pub standard: Option<String>,
    pub price: f64,
    pub description: Option<String>,
}

/// Flat product row as stored in SQLite (no nested variations).
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct ProductRow {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub image: Option<String>,
}

/// Full product with variations — matches server's ProductRead JSON.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ProductData {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub image: Option<String>,
    #[serde(default)]
    pub variations: Vec<VariationData>,
}

/// Payload sent as a JSON string in the multipart "data" field when creating a product.
#[derive(Serialize, Debug, Clone)]
pub struct NewProductPayload {
    pub name: String,
    pub category: String,
    pub variations: Vec<NewVariationPayload>,
}

#[derive(Serialize, Debug, Clone)]
pub struct NewVariationPayload {
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packaging: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
}

/// A single variation to be created with a new product.
#[derive(Debug, Clone)]
pub struct NewVariationInput {
    pub dimensions: Option<String>,
    pub packaging: Option<String>,
    pub standard: Option<String>,
    pub description: Option<String>,
    pub price: f64,
}

/// Used to create a new product (possibly with multiple variations).
#[derive(Debug, Clone)]
pub struct NewProduct {
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub image_path: Option<String>,
    pub variations: Vec<NewVariationInput>,
}
