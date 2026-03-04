#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod models;

use reqwest::Client;
use std::error::Error;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let ui = Main::new()?;
    let client = Client::builder().cookie_store(true).build()?;

    let ui_weak = ui.as_weak();
    let client_clone = client.clone();

    ui.on_login(move |username, password| {
        let ui_weak = ui_weak.clone();
        let client = client_clone.clone();
        let username = username.to_string();
        let password = password.to_string();

        tokio::spawn(async move {
            match auth::login::login(
                &client,
                "https://warehouse.sudurasimontaj.com/user/token",
                &username,
                &password,
            )
            .await
            {
                Ok(response) if response.token.is_some() => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_logged(true);
                        }
                    });
                }
                Ok(response) => {
                    eprintln!("Login failed: {:?}", response.message);
                }
                Err(e) => {
                    eprintln!("Request error: {e}");
                }
            }
        });
    });

    ui.run()?;

    Ok(())
}
