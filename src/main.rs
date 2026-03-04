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
    let me_url = format!("{}/user/me", cfg.base_url);
    let products_url = format!("{}/products/stream", cfg.base_url);
    let base_url = cfg.base_url.clone();
    let stored_token = cfg.token.clone();

    let ui = Main::new()?;

    let cookie_jar = Arc::new(Jar::default());
    let client = Arc::new(
        Client::builder()
            .cookie_provider(cookie_jar.clone())
            .build()
            .expect("Failed to build HTTP client"),
    );

    // Verify stored token on startup
    if let Some(old_token) = stored_token {
        let ui_weak = ui.as_weak();
        let client_check = (*client).clone();
        let base_url_check = base_url.clone();
        let token = old_token.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(auth::login::check_login(&client_check, &me_url, &token));

            let _ = ui_weak.upgrade_in_event_loop(move |ui| match result {
                Ok(new_token) => {
                    // Keep old token or save the rotated one if server returned a new one
                    let token_to_save = new_token.unwrap_or(old_token);
                    let cfg = config::Config { base_url: base_url_check, token: Some(token_to_save) };
                    let _ = cfg.save();
                    ui.set_logged(true);
                }
                Err(e) => {
                    eprintln!("[auth] token invalid: {e}");
                    let cfg = config::Config { base_url: base_url_check, token: None };
                    let _ = cfg.save();
                }
            });
        });
    }

    // Login callback
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();

    ui.on_login(move |username, password| {
        let ui_weak = ui_weak.clone();
        let client = client_clone.clone();
        let username = username.to_string();
        let password = password.to_string();
        let url = login_url.clone();
        let base_url = base_url.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result =
                rt.block_on(auth::login::login(&client, &url, &username, &password));

            let _ = ui_weak.upgrade_in_event_loop(move |ui| match result {
                Ok(r) if r.access_token.is_some() => {
                    let token = r.access_token.unwrap();
                    let cfg = config::Config { base_url, token: Some(token) };
                    if let Err(e) = cfg.save() {
                        eprintln!("[config] failed to save token: {e}");
                    }
                    ui.set_logged(true);
                }
                Ok(r) => eprintln!("[login] rejected — message: {:?}", r.message),
                Err(e) => eprintln!("[login] error: {e}"),
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
