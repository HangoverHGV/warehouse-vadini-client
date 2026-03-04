use sqlx::SqlitePool;

use crate::models::product::{ProductData, ProductRow, VariationData};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct PendingProduct {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub price: f64,
    pub description: Option<String>,
    pub temp_image_path: String,
}

pub async fn upsert(pool: &SqlitePool, p: &ProductData) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO products (id, name, category, image) VALUES (?, ?, ?, ?)",
    )
    .bind(p.id)
    .bind(&p.name)
    .bind(&p.category)
    .bind(&p.image)
    .execute(pool)
    .await?;

    // Rebuild variations for this product
    sqlx::query("DELETE FROM variations WHERE product_id = ?")
        .bind(p.id)
        .execute(pool)
        .await?;

    for v in &p.variations {
        sqlx::query(
            "INSERT OR REPLACE INTO variations
             (id, product_id, dimensions, packaging, standard, price, description)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(v.id)
        .bind(v.product_id)
        .bind(&v.dimensions)
        .bind(&v.packaging)
        .bind(&v.standard)
        .bind(v.price)
        .bind(&v.description)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM products WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn all(pool: &SqlitePool) -> Result<Vec<ProductData>, sqlx::Error> {
    let rows: Vec<ProductRow> =
        sqlx::query_as::<_, ProductRow>("SELECT id, name, category, image FROM products")
            .fetch_all(pool)
            .await?;

    let variations: Vec<VariationData> = sqlx::query_as::<_, VariationData>(
        "SELECT id, product_id, dimensions, packaging, standard, price, description FROM variations",
    )
    .fetch_all(pool)
    .await?;

    let products = rows
        .into_iter()
        .map(|r| {
            let vars = variations
                .iter()
                .filter(|v| v.product_id == r.id)
                .cloned()
                .collect();
            ProductData {
                id: r.id,
                name: r.name,
                category: r.category,
                image: r.image,
                variations: vars,
            }
        })
        .collect();

    Ok(products)
}

pub async fn all_pending(pool: &SqlitePool) -> Result<Vec<PendingProduct>, sqlx::Error> {
    sqlx::query_as::<_, PendingProduct>(
        "SELECT id, name, category, price, description, temp_image_path FROM pending_products",
    )
    .fetch_all(pool)
    .await
}

pub async fn save_pending(
    pool: &SqlitePool,
    name: &str,
    category: &str,
    price: f64,
    description: Option<&str>,
    temp_image_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO pending_products (name, category, price, description, temp_image_path)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(name)
    .bind(category)
    .bind(price)
    .bind(description)
    .bind(temp_image_path)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_pending(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM pending_products WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
