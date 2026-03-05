use sqlx::SqlitePool;

pub async fn create_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS products (
            id          INTEGER PRIMARY KEY,
            name        TEXT NOT NULL DEFAULT '',
            category    TEXT DEFAULT '',
            image       TEXT DEFAULT NULL,
            created_at  TEXT DEFAULT NULL,
            updated_at  TEXT DEFAULT NULL
        )",
    )
    .execute(pool)
    .await?;

    // Migrate existing DBs that don't yet have these columns (ignore error if already present)
    let _ = sqlx::query("ALTER TABLE products ADD COLUMN created_at TEXT DEFAULT NULL")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE products ADD COLUMN updated_at TEXT DEFAULT NULL")
        .execute(pool)
        .await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS variations (
            id          INTEGER PRIMARY KEY,
            product_id  INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
            dimensions  TEXT DEFAULT NULL,
            packaging   TEXT DEFAULT NULL,
            standard    TEXT DEFAULT NULL,
            price       REAL NOT NULL DEFAULT 0.0,
            description TEXT DEFAULT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pending_products (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT DEFAULT '',
            category        TEXT DEFAULT '',
            price           REAL DEFAULT 0.0,
            description     TEXT DEFAULT NULL,
            temp_image_path TEXT DEFAULT '',
            created_at      TEXT DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}
