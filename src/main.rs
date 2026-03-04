#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod config;
mod models;
mod sync;

use reqwest::{cookie::Jar, Client};
use std::sync::Arc;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let cfg = config::Config::load().expect("Failed to load config");
    let login_url = format!("{}/user/token", cfg.base_url);
    let products_url = format!("{}/products/stream", cfg.base_url);

    let ui = Main::new()?;

    let cookie_jar = Arc::new(Jar::default());
    let client = Arc::new(
        Client::builder()
            .cookie_provider(cookie_jar.clone())
            .build()
            .expect("Failed to build HTTP client"),
    );

    // Login callback
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();

    ui.on_login(move |username, password| {
        let ui_weak = ui_weak.clone();
        let client = client_clone.clone();
        let username = username.to_string();
        let password = password.to_string();
        let url = login_url.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result =
                rt.block_on(auth::login::login(&client, &url, &username, &password));

            let _ = ui_weak.upgrade_in_event_loop(move |ui| match result {
                Ok(r) if r.token.is_some() => ui.set_logged(true),
                Ok(r) => eprintln!("Login failed: {:?}", r.message),
                Err(e) => eprintln!("Request error: {e}"),
            });
        });
    });

    // SSE product sync — runs in a background thread
    let ui_weak2 = ui.as_weak();
    let client2 = (*client).clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(sync::listen_for_changes(client2, products_url, ui_weak2));
    });

    ui.run()
}
