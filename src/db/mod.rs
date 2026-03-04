pub mod products;
pub mod schema;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::path::Path;

pub async fn open(data_dir: &Path) -> Result<SqlitePool, sqlx::Error> {
    tokio::fs::create_dir_all(data_dir).await.ok();
    tokio::fs::create_dir_all(data_dir.join("images")).await.ok();
    tokio::fs::create_dir_all(data_dir.join("temp")).await.ok();

    let opts = SqliteConnectOptions::new()
        .filename(data_dir.join("db.sqlite"))
        .create_if_missing(true)
        .pragma("foreign_keys", "ON");

    let pool = SqlitePool::connect_with(opts).await?;
    schema::create_tables(&pool).await?;
    Ok(pool)
}
