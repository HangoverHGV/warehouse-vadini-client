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
        "INSERT OR REPLACE INTO products (id, name, category, image, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(p.id)
    .bind(&p.name)
    .bind(&p.category)
    .bind(&p.image)
    .bind(&p.created_at)
    .bind(&p.updated_at)
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
        sqlx::query_as::<_, ProductRow>("SELECT id, name, category, image, created_at, updated_at FROM products")
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
                created_at: r.created_at,
                updated_at: r.updated_at,
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

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct VariationWithProduct {
    pub variation_id: i64,
    pub product_name: String,
    pub dimensions: Option<String>,
    pub packaging: Option<String>,
    pub standard: Option<String>,
    pub price: f64,
}

pub async fn get_variation_by_id(
    pool: &SqlitePool,
    variation_id: i64,
) -> Result<Option<VariationWithProduct>, sqlx::Error> {
    sqlx::query_as::<_, VariationWithProduct>(
        "SELECT v.id as variation_id, p.name as product_name,
                v.dimensions, v.packaging, v.standard, v.price
         FROM variations v
         JOIN products p ON v.product_id = p.id
         WHERE v.id = ?",
    )
    .bind(variation_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<Option<ProductData>, sqlx::Error> {
    let row: Option<ProductRow> = sqlx::query_as::<_, ProductRow>(
        "SELECT id, name, category, image, created_at, updated_at FROM products WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let r = match row {
        None => return Ok(None),
        Some(r) => r,
    };

    let variations: Vec<VariationData> = sqlx::query_as::<_, VariationData>(
        "SELECT id, product_id, dimensions, packaging, standard, price, description FROM variations WHERE product_id = ?",
    )
    .bind(r.id)
    .fetch_all(pool)
    .await?;

    Ok(Some(ProductData {
        id: r.id,
        name: r.name,
        category: r.category,
        image: r.image,
        variations,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }))
}
