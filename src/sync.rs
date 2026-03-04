use reqwest::Client;
use std::rc::Rc;

use crate::models::product::ProductData;
use crate::{Main, ProductDetails};

pub async fn listen_for_changes(client: Client, url: String, ui_handle: slint::Weak<Main>) {
    let mut response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("SSE connect error: {e}");
            return;
        }
    };

    let mut buffer = String::new();

    while let Ok(Some(chunk)) = response.chunk().await {
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        for line in buffer.lines() {
            if let Some(json) = line.strip_prefix("data: ") {
                if let Ok(products) = serde_json::from_str::<Vec<ProductData>>(json) {
                    // Vec<ProductData> is Send; convert to ProductDetails (which contains
                    // slint::Image, not Send) only inside the event loop closure.
                    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                        let slint_products: Vec<ProductDetails> = products
                            .into_iter()
                            .map(|p| ProductDetails {
                                title: p.title.into(),
                                description: p.description.into(),
                                category: p.category.into(),
                                price: p.price.into(),
                                image: Default::default(),
                            })
                            .collect();
                        let model = Rc::new(slint::VecModel::from(slint_products));
                        ui.set_products(model.into());
                    });
                }
            }
        }

        buffer.clear();
    }
}
