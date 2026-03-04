use reqwest::Client;
use std::path::{Path, PathBuf};

/// Returns the local cache path for an image ID (stored as PNG).
pub fn local_path(data_dir: &Path, image_id: &str) -> PathBuf {
    data_dir.join("images").join(format!("{image_id}_medium.png"))
}

/// Returns the local temp path for an offline-staged image.
pub fn temp_path(data_dir: &Path, filename: &str) -> PathBuf {
    data_dir.join("temp").join(filename)
}

/// Downloads the medium image for `image_id` if not already cached.
/// URL: `{base_url}/images/medium/{image_id}_medium.webp`
pub async fn ensure(
    client: &Client,
    base_url: &str,
    data_dir: &Path,
    image_id: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let path = local_path(data_dir, image_id);

    if path.exists() {
        return Ok(path);
    }

    let url = format!("{base_url}/images/medium/{image_id}_medium.webp");
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(format!("Image fetch failed: {}", response.status()).into());
    }

    let bytes = response.bytes().await?;

    // Decode WebP and re-encode as PNG (slint doesn't support WebP natively)
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::WebP)?;
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    img.save_with_format(&path, image::ImageFormat::Png)?;

    eprintln!("[images] cached {image_id}_medium.png");
    Ok(path)
}
