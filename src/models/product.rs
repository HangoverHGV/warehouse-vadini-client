use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ProductData {
    pub title: String,
    pub description: String,
    pub category: String,
    pub price: String,
}
