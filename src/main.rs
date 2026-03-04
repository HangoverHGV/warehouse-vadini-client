#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod auth;
mod config;
mod db;
mod images;
mod models;
mod sync;

use reqwest::{cookie::Jar, Client};
use sqlx::SqlitePool;
use std::sync::Arc;

slint::include_modules!();

fn start_sync(
    rt: Arc<tokio::runtime::Runtime>,
    client: Client,
    pool: SqlitePool,
    base_url: String,
    token: String,
    data_dir: std::path::PathBuf,
    ui_handle: slint::Weak<Main>,
) {
    std::thread::spawn(move || {
        rt.block_on(async move {
            if let Err(e) =
                sync::push_pending(&client, &base_url, &token, &pool, &data_dir).await
            {
                eprintln!("[main] push_pending: {e}");
            }
            if let Err(e) =
                sync::initial_sync(&client, &base_url, &token, &pool, &data_dir, ui_handle.clone())
                    .await
            {
                eprintln!("[main] initial_sync: {e}");
            }
            sync::listen_for_changes(client, base_url, token, pool, data_dir, ui_handle).await;
        });
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let cfg = config::Config::load().expect("Failed to load config");
    let base_url = cfg.base_url.clone();
    let stored_token = cfg.token.clone();

    let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
    let data_dir = config::Config::data_dir();
    let pool = rt.block_on(db::open(&data_dir)).expect("Failed to open DB");

    let ui = Main::new()?;

    let client = Arc::new(
        Client::builder()
            .cookie_provider(Arc::new(Jar::default()))
            .build()
            .expect("Failed to build HTTP client"),
    );

    let login_url = format!("{base_url}/user/token");
    let me_url = format!("{base_url}/user/me");

    // Verify stored token on startup
    if let Some(old_token) = stored_token {
        let ui_weak = ui.as_weak();
        let client2 = (*client).clone();
        let base_url2 = base_url.clone();
        let token = old_token.clone();
        let rt2 = rt.clone();
        let pool2 = pool.clone();
        let data_dir2 = data_dir.clone();

        std::thread::spawn(move || {
            let result = rt2.block_on(auth::login::check_login(&client2, &me_url, &token));

            match result {
                Ok(new_token) => {
                    let final_token = new_token.unwrap_or(old_token);
                    let cfg = config::Config {
                        base_url: base_url2.clone(),
                        token: Some(final_token.clone()),
                    };
                    let _ = cfg.save();
                    let _ = ui_weak.clone().upgrade_in_event_loop(|ui| ui.set_logged(true));
                    start_sync(rt2, client2, pool2, base_url2, final_token, data_dir2, ui_weak);
                }
                Err(e) => {
                    eprintln!("[auth] token invalid: {e}");
                    let cfg = config::Config { base_url: base_url2, token: None };
                    let _ = cfg.save();
                }
            }
        });
    }

    // Login callback
    let ui_weak = ui.as_weak();
    let client_login = client.clone();
    let rt_login = rt.clone();
    let pool_login = pool.clone();
    let data_dir_login = data_dir.clone();
    let base_url_login = base_url.clone();

    ui.on_login(move |username, password| {
        let ui_weak = ui_weak.clone();
        let client = (*client_login).clone();
        let username = username.to_string();
        let password = password.to_string();
        let url = login_url.clone();
        let base_url = base_url_login.clone();
        let rt = rt_login.clone();
        let pool = pool_login.clone();
        let data_dir = data_dir_login.clone();

        std::thread::spawn(move || {
            let result = rt.block_on(auth::login::login(&client, &url, &username, &password));

            match result {
                Ok(r) if r.access_token.is_some() => {
                    let token = r.access_token.unwrap();
                    let cfg = config::Config {
                        base_url: base_url.clone(),
                        token: Some(token.clone()),
                    };
                    if let Err(e) = cfg.save() {
                        eprintln!("[config] failed to save token: {e}");
                    }
                    let _ = ui_weak.clone().upgrade_in_event_loop(|ui| ui.set_logged(true));
                    start_sync(rt, client, pool, base_url, token, data_dir, ui_weak);
                }
                Ok(r) => eprintln!("[login] rejected — message: {:?}", r.message),
                Err(e) => eprintln!("[login] error: {e}"),
            }
        });
    });

    ui.run()
}
