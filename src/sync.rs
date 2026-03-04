use reqwest::Client;
use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::models::product::{NewProduct, NewVariationInput, ProductData};
use crate::{api, db, images, Main, ProductDetails};

type SyncError = Box<dyn std::error::Error + Send + Sync>;

/// Shared in-memory cache of all products from DB.
pub type ProductCache = Arc<Mutex<Vec<ProductData>>>;

fn load_image(data_dir: &Path, image_path: &str) -> slint::Image {
    let path = images::local_path(data_dir, image_path);
    if path.exists() {
        slint::Image::load_from_path(&path).unwrap_or_default()
    } else {
        Default::default()
    }
}

fn price_range(variations: &[crate::models::product::VariationData]) -> String {
    if variations.is_empty() {
        return String::new();
    }
    let mut prices: Vec<f64> = variations.iter().map(|v| v.price).collect();
    prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min = prices.first().unwrap();
    let max = prices.last().unwrap();
    if (max - min).abs() < 0.005 {
        format!("{min:.2} RON")
    } else {
        format!("{min:.2} - {max:.2} RON")
    }
}

fn to_slint(p: ProductData, data_dir: &Path) -> ProductDetails {
    let img = p
        .image
        .as_deref()
        .map(|path| load_image(data_dir, path))
        .unwrap_or_default();
    ProductDetails {
        id: p.id as i32,
        title: p.name.into(),
        description: String::new().into(),
        category: p.category.into(),
        price: price_range(&p.variations).into(),
        image: img,
    }
}

/// Called on the UI thread — reads current search/filter from UI and updates products + categories.
pub fn apply_filter_on_ui_thread(ui: &Main, products: &[ProductData], data_dir: &Path) {
    let query = ui.get_search_query().to_string().to_lowercase();
    let category = ui.get_filter_category().to_string();

    // Build sorted categories list
    let mut cat_set: BTreeSet<String> = BTreeSet::new();
    for p in products {
        cat_set.insert(p.category.clone());
    }
    let cats: Vec<slint::SharedString> = std::iter::once(slint::SharedString::from("Tot"))
        .chain(cat_set.into_iter().map(slint::SharedString::from))
        .collect();
    ui.set_categories(Rc::new(slint::VecModel::from(cats)).into());

    // Filter and convert
    let filtered: Vec<ProductDetails> = products
        .iter()
        .filter(|p| {
            let name_ok = query.is_empty() || p.name.to_lowercase().contains(&query);
            let cat_ok = category.is_empty() || category == "Tot" || p.category == category;
            name_ok && cat_ok
        })
        .map(|p| to_slint(p.clone(), data_dir))
        .collect();

    ui.set_products(Rc::new(slint::VecModel::from(filtered)).into());
}

/// Called from background threads — schedules filter application via event loop.
pub fn apply_filter(products: Vec<ProductData>, data_dir: PathBuf, ui_handle: slint::Weak<Main>) {
    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
        apply_filter_on_ui_thread(&ui, &products, &data_dir);
    });
}

/// Update cache and re-render with current filter.
pub fn refresh_ui(
    products: Vec<ProductData>,
    cache: ProductCache,
    data_dir: PathBuf,
    ui_handle: slint::Weak<Main>,
) {
    *cache.lock().unwrap() = products.clone();
    apply_filter(products, data_dir, ui_handle);
}

pub async fn initial_sync(
    client: &Client,
    base_url: &str,
    token: &str,
    pool: &SqlitePool,
    data_dir: &Path,
    cache: ProductCache,
    ui_handle: slint::Weak<Main>,
) -> Result<(), SyncError> {
    eprintln!("[sync] initial_sync start");
    let products = api::products::fetch_all(client, base_url, token).await?;

    for p in &products {
        db::products::upsert(pool, p).await?;
        if let Some(ref image_path) = p.image {
            if let Err(e) = images::ensure(client, base_url, data_dir, image_path).await {
                eprintln!("[sync] image error for {}: {e}", p.id);
            }
        }
    }

    let all = db::products::all(pool).await?;
    refresh_ui(all, cache, data_dir.to_path_buf(), ui_handle);
    eprintln!("[sync] initial_sync done ({} products)", products.len());
    Ok(())
}

pub async fn push_pending(
    client: &Client,
    base_url: &str,
    token: &str,
    pool: &SqlitePool,
    data_dir: &Path,
) -> Result<(), SyncError> {
    let pending = db::products::all_pending(pool).await?;
    if pending.is_empty() {
        return Ok(());
    }
    eprintln!("[sync] pushing {} pending products", pending.len());

    for p in pending {
        let new_product = NewProduct {
            name: p.name.clone(),
            category: p.category.clone(),
            description: p.description.clone(),
            image_path: None,
            variations: vec![NewVariationInput {
                dimensions: None,
                packaging: None,
                standard: None,
                description: p.description.clone(),
                price: p.price,
            }],
        };

        match api::products::create(client, base_url, token, &new_product).await {
            Ok(created) => {
                let full = api::products::fetch_one(client, base_url, token, created.id)
                    .await
                    .unwrap_or(created);
                db::products::upsert(pool, &full).await?;
                if let Some(ref image_path) = full.image {
                    if let Err(e) = images::ensure(client, base_url, data_dir, image_path).await {
                        eprintln!("[sync] image error: {e}");
                    }
                }
                if !p.temp_image_path.is_empty() {
                    let _ = tokio::fs::remove_file(&p.temp_image_path).await;
                }
                db::products::delete_pending(pool, p.id).await?;
                eprintln!("[sync] pushed pending id={}", p.id);
            }
            Err(e) => eprintln!("[sync] failed to push pending {}: {e}", p.id),
        }
    }

    Ok(())
}

pub async fn listen_for_changes(
    client: Client,
    base_url: String,
    token: String,
    pool: SqlitePool,
    data_dir: PathBuf,
    cache: ProductCache,
    ui_handle: slint::Weak<Main>,
) {
    let url = format!("{base_url}/sync/stream");
    let mut response = match client.get(&url).bearer_auth(&token).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[sync] SSE connect error: {e}");
            return;
        }
    };

    let mut event_type = String::new();
    let mut data_lines: Vec<String> = Vec::new();

    while let Ok(Some(chunk)) = response.chunk().await {
        let text = String::from_utf8_lossy(&chunk).to_string();

        for line in text.lines() {
            if line.is_empty() {
                if !data_lines.is_empty() {
                    let json = data_lines.join("\n");
                    handle_event(
                        &event_type,
                        &json,
                        &pool,
                        &client,
                        &base_url,
                        &data_dir,
                        cache.clone(),
                        ui_handle.clone(),
                    )
                    .await;
                }
                event_type.clear();
                data_lines.clear();
            } else if let Some(ev) = line.strip_prefix("event: ") {
                event_type = ev.to_string();
            } else if let Some(data) = line.strip_prefix("data: ") {
                data_lines.push(data.to_string());
            }
        }
    }

    eprintln!("[sync] SSE stream ended");
}

async fn handle_event(
    event: &str,
    json: &str,
    pool: &SqlitePool,
    client: &Client,
    base_url: &str,
    data_dir: &Path,
    cache: ProductCache,
    ui_handle: slint::Weak<Main>,
) {
    if event.contains("delete") {
        if let Ok(p) = serde_json::from_str::<ProductData>(json) {
            let _ = db::products::delete(pool, p.id).await;
        }
    } else if let Ok(p) = serde_json::from_str::<ProductData>(json) {
        if !p.name.is_empty() {
            let _ = db::products::upsert(pool, &p).await;
            if let Some(ref image_path) = p.image {
                if let Err(e) = images::ensure(client, base_url, data_dir, image_path).await {
                    eprintln!("[sync] image error: {e}");
                }
            }
        }
    } else if let Ok(products) = serde_json::from_str::<Vec<ProductData>>(json) {
        for p in &products {
            let _ = db::products::upsert(pool, p).await;
            if let Some(ref image_path) = p.image {
                if let Err(e) = images::ensure(client, base_url, data_dir, image_path).await {
                    eprintln!("[sync] image error: {e}");
                }
            }
        }
    }

    if let Ok(all) = db::products::all(pool).await {
        refresh_ui(all, cache, data_dir.to_path_buf(), ui_handle);
    }
}
