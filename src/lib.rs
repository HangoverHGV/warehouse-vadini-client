pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod images;
pub mod models;
pub mod sync;

use reqwest::{cookie::Jar, Client};
use slint::{Model, ModelRc};
use sqlx::SqlitePool;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

slint::include_modules!();

fn price_per_unit(packaging: &str, price: f64) -> String {
    let p = packaging.trim();
    if p.is_empty() {
        return format!("{price:.2} RON");
    }
    let num_end = p.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(p.len());
    let qty: f64 = p[..num_end].parse().unwrap_or(1.0);
    if qty <= 0.0 {
        return format!("{price:.2} RON");
    }
    let unit = p[num_end..].split('/').next().unwrap_or("").trim();
    let per = price / qty;
    if unit.is_empty() { format!("{per:.2} RON") } else { format!("{per:.2} RON/{unit}") }
}

fn to_product_full(p: models::product::ProductData, data_dir: &std::path::Path) -> ProductFull {
    let img = p
        .image
        .as_deref()
        .map(|id| images::local_path(data_dir, id))
        .filter(|path| path.exists())
        .and_then(|path| slint::Image::load_from_path(&path).ok())
        .unwrap_or_default();

    let has_dimensions = p.variations.iter().any(|v| v.dimensions.as_deref().map_or(false, |s| !s.is_empty()));
    let has_packaging   = p.variations.iter().any(|v| v.packaging.as_deref().map_or(false, |s| !s.is_empty()));
    let has_standard    = p.variations.iter().any(|v| v.standard.as_deref().map_or(false, |s| !s.is_empty()));

    let variations: Vec<VariationDetails> = p
        .variations
        .iter()
        .map(|v| VariationDetails {
            id: v.id as i32,
            dimensions:     v.dimensions.clone().unwrap_or_default().into(),
            packaging:      v.packaging.clone().unwrap_or_default().into(),
            standard:       v.standard.clone().unwrap_or_default().into(),
            price:          v.price as f32,
            price_total:    format!("{:.2} RON", v.price).into(),
            price_per_unit: price_per_unit(v.packaging.as_deref().unwrap_or(""), v.price).into(),
        })
        .collect();

    let var_model: ModelRc<VariationDetails> = Rc::new(slint::VecModel::from(variations)).into();

    ProductFull {
        id: p.id as i32,
        name: p.name.into(),
        image: img,
        category: p.category.into(),
        variations: var_model,
        has_dimensions,
        has_packaging,
        has_standard,
        include_in_catalog: p.include_in_catalog,
    }
}

fn cart_total(model: &slint::VecModel<CartItem>) -> String {
    let total: f64 = (0..model.row_count())
        .filter_map(|i| model.row_data(i))
        .map(|item| item.price as f64 * item.quantity as f64)
        .sum();
    format!("{total:.2} RON")
}

fn opt_str(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}

fn start_sync(
    rt: Arc<tokio::runtime::Runtime>,
    client: Client,
    pool: SqlitePool,
    base_url: String,
    token: String,
    data_dir: std::path::PathBuf,
    cache: sync::ProductCache,
    ui_handle: slint::Weak<Main>,
) {
    std::thread::spawn(move || {
        rt.block_on(async move {
            if let Err(e) = sync::push_pending(&client, &base_url, &token, &pool, &data_dir).await {
                eprintln!("[main] push_pending: {e}");
            }
            if let Err(e) = sync::initial_sync(
                &client, &base_url, &token, &pool, &data_dir,
                cache.clone(), ui_handle.clone(),
            ).await {
                eprintln!("[main] initial_sync: {e}");
            }
            sync::listen_for_changes(client, base_url, token, pool, data_dir, cache, ui_handle).await;
        });
    });
}

async fn check_admin(client: &Client, base_url: &str, token: &str) -> bool {
    let res = client.get(format!("{base_url}/user/")).bearer_auth(token).send().await;
    matches!(res, Ok(r) if r.status().is_success())
}

pub fn run_app() -> Result<(), slint::PlatformError> {
    let cfg = config::Config::load();
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

    let shared_token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(stored_token.clone()));
    let cache: sync::ProductCache = Arc::new(Mutex::new(vec![]));

    let cart = Rc::new(slint::VecModel::<CartItem>::from(vec![]));
    ui.set_cart_items(ModelRc::new(cart.clone()));

    // Initialize create-variations model with one empty row
    let create_vars_model = Rc::new(slint::VecModel::<VariationInput>::from(vec![
        VariationInput::default(),
    ]));
    ui.set_create_variations(ModelRc::new(create_vars_model.clone()));

    ui.set_server_url(base_url.clone().into());

    let me_url = format!("{base_url}/user/me");

    // --- Verify stored token on startup ---
    if let Some(old_token) = stored_token {
        // Immediately show the app using cached credentials — no login flash
        ui.set_logged(true);
        ui.set_is_admin(cfg.is_admin);

        let ui_weak = ui.as_weak();
        let client2 = (*client).clone();
        let base_url2 = base_url.clone();
        let token = old_token.clone();
        let rt2 = rt.clone();
        let pool2 = pool.clone();
        let data_dir2 = data_dir.clone();
        let shared_token2 = shared_token.clone();
        let cache2 = cache.clone();

        std::thread::spawn(move || {
            let result = rt2.block_on(auth::login::check_login(&client2, &me_url, &token));
            match result {
                Ok(new_token) => {
                    let final_token = new_token.unwrap_or(old_token);
                    let is_admin = rt2.block_on(check_admin(&client2, &base_url2, &final_token));
                    let _ = config::Config { base_url: base_url2.clone(), token: Some(final_token.clone()), is_admin }.save();
                    *shared_token2.lock().unwrap() = Some(final_token.clone());
                    let _ = ui_weak.clone().upgrade_in_event_loop(move |ui| {
                        ui.set_is_admin(is_admin);
                    });
                    start_sync(rt2, client2, pool2, base_url2, final_token, data_dir2, cache2, ui_weak);
                }
                Err(e) => {
                    let is_network_err = e
                        .downcast_ref::<reqwest::Error>()
                        .map(|re| re.is_connect() || re.is_timeout())
                        .unwrap_or(false);
                    if is_network_err {
                        // No network — stay logged in with cached data
                        eprintln!("[auth] no network, staying logged in offline");
                        *shared_token2.lock().unwrap() = Some(old_token);
                    } else {
                        // Token truly rejected — force re-login
                        eprintln!("[auth] token invalid: {e}");
                        let _ = config::Config { base_url: base_url2, token: None, is_admin: false }.save();
                        let _ = ui_weak.upgrade_in_event_loop(|ui| {
                            ui.set_logged(false);
                            ui.set_is_admin(false);
                        });
                    }
                }
            }
        });
    }

    // --- Login callback ---
    {
        let ui_weak = ui.as_weak();
        let client_l = client.clone();
        let rt_l = rt.clone();
        let pool_l = pool.clone();
        let data_dir_l = data_dir.clone();
        let base_url_l = base_url.clone();
        let shared_token_l = shared_token.clone();
        let cache_l = cache.clone();

        ui.on_login(move |server_url, username, password| {
            let ui_weak = ui_weak.clone();
            let client = (*client_l).clone();
            let base_url = if server_url.is_empty() {
                base_url_l.clone()
            } else {
                server_url.trim_end_matches('/').to_string()
            };
            let url = format!("{base_url}/user/token");
            let rt = rt_l.clone();
            let pool = pool_l.clone();
            let data_dir = data_dir_l.clone();
            let shared_token = shared_token_l.clone();
            let cache = cache_l.clone();
            let username = username.to_string();
            let password = password.to_string();

            std::thread::spawn(move || {
                let result = rt.block_on(auth::login::login(&client, &url, &username, &password));
                match result {
                    Ok(r) if r.access_token.is_some() => {
                        let token = r.access_token.unwrap();
                        let is_admin = rt.block_on(check_admin(&client, &base_url, &token));
                        let _ = config::Config { base_url: base_url.clone(), token: Some(token.clone()), is_admin }.save();
                        *shared_token.lock().unwrap() = Some(token.clone());
                        let _ = ui_weak.clone().upgrade_in_event_loop(move |ui| {
                            ui.set_logged(true);
                            ui.set_is_admin(is_admin);
                        });
                        start_sync(rt, client, pool, base_url, token, data_dir, cache, ui_weak);
                    }
                    Ok(r) => eprintln!("[login] rejected: {:?}", r.message),
                    Err(e) => eprintln!("[login] error: {e:#}"),
                }
            });
        });
    }

    // --- open-product ---
    {
        let ui_weak = ui.as_weak();
        let pool_op = pool.clone();
        let data_dir_op = data_dir.clone();
        let rt_op = rt.clone();

        ui.on_open_product(move |product_id| {
            let ui_weak = ui_weak.clone();
            let pool = pool_op.clone();
            let data_dir = data_dir_op.clone();
            let rt = rt_op.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    match db::products::get_by_id(&pool, product_id as i64).await {
                        Ok(Some(p)) => {
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                let full = to_product_full(p, &data_dir);
                                ui.set_selected_product(full);
                                ui.set_current_view(1);
                            });
                        }
                        Ok(None) => eprintln!("[main] product {product_id} not found"),
                        Err(e) => eprintln!("[main] get_by_id error: {e}"),
                    }
                });
            });
        });
    }

    // --- search (UI thread) ---
    {
        let cache_s = cache.clone();
        let data_dir_s = data_dir.clone();
        let ui_weak_s = ui.as_weak();

        ui.on_search(move |query| {
            let ui = ui_weak_s.upgrade().unwrap();
            ui.set_search_query(query.clone());
            let products = cache_s.lock().unwrap().clone();
            sync::apply_filter_on_ui_thread(&ui, &products, &data_dir_s);
        });
    }

    // --- filter-changed (UI thread) ---
    {
        let cache_f = cache.clone();
        let data_dir_f = data_dir.clone();
        let ui_weak_f = ui.as_weak();

        ui.on_filter_changed(move |category| {
            let ui = ui_weak_f.upgrade().unwrap();
            ui.set_filter_category(category.clone());
            let products = cache_f.lock().unwrap().clone();
            sync::apply_filter_on_ui_thread(&ui, &products, &data_dir_f);
        });
    }

    // --- add-to-cart ---
    {
        let cart_add = cart.clone();
        let ui_weak = ui.as_weak();

        ui.on_add_to_cart(move |variation_id| {
            let ui = ui_weak.upgrade().unwrap();
            let selected = ui.get_selected_product();

            for i in 0..selected.variations.row_count() {
                let v = match selected.variations.row_data(i) {
                    Some(v) if v.id == variation_id => v,
                    _ => continue,
                };

                for j in 0..cart_add.row_count() {
                    if let Some(mut item) = cart_add.row_data(j) {
                        if item.variation_id == variation_id {
                            item.quantity += 1;
                            cart_add.set_row_data(j, item);
                            ui.set_cart_count(cart_add.row_count() as i32);
                            ui.set_cart_total(cart_total(&cart_add).into());
                            return;
                        }
                    }
                }

                cart_add.push(CartItem {
                    variation_id: v.id,
                    product_name: selected.name.clone(),
                    dimensions: v.dimensions.clone(),
                    packaging: v.packaging.clone(),
                    standard: v.standard.clone(),
                    price: v.price_total.as_str().split_whitespace().next()
                        .and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0),
                    quantity: 1,
                    item_discount: 0.0,
                });
                ui.set_cart_count(cart_add.row_count() as i32);
                ui.set_cart_total(cart_total(&cart_add).into());
                break;
            }
        });
    }

    // --- remove-from-cart ---
    {
        let cart_rm = cart.clone();
        let ui_weak = ui.as_weak();

        ui.on_remove_from_cart(move |variation_id| {
            for i in 0..cart_rm.row_count() {
                if let Some(item) = cart_rm.row_data(i) {
                    if item.variation_id == variation_id {
                        cart_rm.remove(i);
                        break;
                    }
                }
            }
            let ui = ui_weak.upgrade().unwrap();
            ui.set_cart_count(cart_rm.row_count() as i32);
            ui.set_cart_total(cart_total(&cart_rm).into());
        });
    }

    // --- update-quantity ---
    {
        let cart_uq = cart.clone();
        let ui_weak = ui.as_weak();

        ui.on_update_quantity(move |variation_id, qty| {
            for i in 0..cart_uq.row_count() {
                if let Some(mut item) = cart_uq.row_data(i) {
                    if item.variation_id == variation_id {
                        item.quantity = qty;
                        cart_uq.set_row_data(i, item);
                        break;
                    }
                }
            }
            let ui = ui_weak.upgrade().unwrap();
            ui.set_cart_total(cart_total(&cart_uq).into());
        });
    }

    // --- clear-cart ---
    {
        let cart_cl = cart.clone();
        let ui_weak = ui.as_weak();

        ui.on_clear_cart(move || {
            while cart_cl.row_count() > 0 { cart_cl.remove(0); }
            let ui = ui_weak.upgrade().unwrap();
            ui.set_cart_count(0);
            ui.set_cart_total("0.00 RON".into());
        });
    }

    // --- send-order ---
    {
        let cart_so = cart.clone();
        let ui_weak = ui.as_weak();
        let client_so = client.clone();
        let rt_so = rt.clone();
        let base_url_so = base_url.clone();
        let shared_token_so = shared_token.clone();

        ui.on_send_order(move |client_name, addr_invoice, addr_delivery, discount| {
            let token = match shared_token_so.lock().unwrap().clone() {
                Some(t) => t,
                None => { eprintln!("[order] no token"); return; }
            };

            let items: Vec<api::orders::OrderItemCreate> = (0..cart_so.row_count())
                .filter_map(|i| cart_so.row_data(i))
                .map(|item| api::orders::OrderItemCreate {
                    variation_id: item.variation_id as i64,
                    quantity: item.quantity as i64,
                    discount: item.item_discount as f64,
                })
                .collect();

            if items.is_empty() { eprintln!("[order] cart is empty"); return; }

            let order = api::orders::OrderCreate {
                client_name: client_name.to_string(),
                address_invoice: addr_invoice.to_string(),
                address_delivery: addr_delivery.to_string(),
                discount: discount as f64,
                items,
            };

            let client = (*client_so).clone();
            let base_url = base_url_so.clone();
            let rt = rt_so.clone();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    match api::orders::create(&client, &base_url, &token, order).await {
                        Ok(()) => {
                            eprintln!("[order] sent successfully");
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                ui.invoke_clear_cart();
                                ui.set_current_view(0);
                            });
                        }
                        Err(e) => eprintln!("[order] failed: {e}"),
                    }
                });
            });
        });
    }

    // --- variation row management (create form) ---
    {
        let model = create_vars_model.clone();
        ui.on_add_variation_row(move || {
            model.push(VariationInput::default());
        });
    }
    {
        let model = create_vars_model.clone();
        ui.on_remove_variation_row(move |i| {
            let idx = i as usize;
            if idx < model.row_count() {
                model.remove(idx);
            }
        });
    }
    {
        let model = create_vars_model.clone();
        ui.on_variation_field_changed(move |i, dims, pack, std_val, price| {
            let idx = i as usize;
            if idx < model.row_count() {
                model.set_row_data(idx, VariationInput {
                    dims: dims.clone(),
                    pack: pack.clone(),
                    std: std_val.clone(),
                    price: price.clone(),
                });
            }
        });
    }
    {
        let model = create_vars_model.clone();
        ui.on_clear_create_variations(move || {
            model.set_vec(vec![VariationInput::default()]);
        });
    }

    // --- create-product ---
    {
        let client_cp = client.clone();
        let rt_cp = rt.clone();
        let base_url_cp = base_url.clone();
        let shared_token_cp = shared_token.clone();
        let pool_cp = pool.clone();
        let data_dir_cp = data_dir.clone();
        let cache_cp = cache.clone();
        let ui_weak = ui.as_weak();
        let model_cp = create_vars_model.clone();
        ui.on_create_product(move |name, category, desc, img_path| {
            let token = match shared_token_cp.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_cp).clone();
            let base_url = base_url_cp.clone();
            let rt = rt_cp.clone();
            let pool = pool_cp.clone();
            let data_dir = data_dir_cp.clone();
            let cache = cache_cp.clone();
            let ui_weak = ui_weak.clone();

            let variations: Vec<models::product::NewVariationInput> = (0..model_cp.row_count())
                .filter_map(|i| model_cp.row_data(i))
                .map(|v| models::product::NewVariationInput {
                    dimensions: opt_str(&v.dims),
                    packaging: opt_str(&v.pack),
                    standard: opt_str(&v.std),
                    description: None,
                    price: v.price.parse::<f64>().unwrap_or(0.0),
                })
                .collect();

            let product = models::product::NewProduct {
                name: name.to_string(),
                category: category.to_string(),
                description: opt_str(&desc),
                image_path: opt_str(&img_path),
                variations,
            };

            std::thread::spawn(move || {
                rt.block_on(async move {
                    match api::products::create(&client, &base_url, &token, &product).await {
                        Ok(created) => {
                            eprintln!("[main] product created id={}", created.id);
                            let _ = db::products::upsert(&pool, &created).await;
                            if let Ok(all) = db::products::all(&pool).await {
                                sync::refresh_ui(all, cache, data_dir, ui_weak.clone());
                            }
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                ui.set_create_image_path("".into());
                                ui.invoke_clear_create_variations();
                                ui.set_current_view(0);
                            });
                        }
                        Err(e) => eprintln!("[main] create_product error: {e}"),
                    }
                });
            });
        });
    }

    // --- select-image-create (desktop only) ---
    #[cfg(not(target_os = "android"))]
    {
        let ui_weak = ui.as_weak();
        ui.on_select_image_create(move || {
            let ui_weak = ui_weak.clone();
            std::thread::spawn(move || {
                let file = rfd::FileDialog::new()
                    .add_filter("Images", &["jpg", "jpeg", "png", "webp"])
                    .pick_file();
                if let Some(path) = file {
                    let path_str = path.to_string_lossy().to_string();
                    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_create_image_path(path_str.into());
                    });
                }
            });
        });
    }

    // --- change-product-image (desktop only) ---
    #[cfg(not(target_os = "android"))]
    {
        let client_cpi = client.clone();
        let rt_cpi = rt.clone();
        let base_url_cpi = base_url.clone();
        let shared_token_cpi = shared_token.clone();
        let pool_cpi = pool.clone();
        let data_dir_cpi = data_dir.clone();
        let ui_weak = ui.as_weak();

        ui.on_change_product_image(move |product_id| {
            let client = (*client_cpi).clone();
            let base_url = base_url_cpi.clone();
            let rt = rt_cpi.clone();
            let pool = pool_cpi.clone();
            let data_dir = data_dir_cpi.clone();
            let ui_weak = ui_weak.clone();
            let token = match shared_token_cpi.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };

            std::thread::spawn(move || {
                let file = rfd::FileDialog::new()
                    .add_filter("Images", &["jpg", "jpeg", "png", "webp"])
                    .pick_file();
                if let Some(path) = file {
                    let path_str = path.to_string_lossy().to_string();
                    rt.block_on(async move {
                        let name = match db::products::get_by_id(&pool, product_id as i64).await {
                            Ok(Some(p)) => p.name,
                            _ => String::new(),
                        };
                        match api::products::update_product_image(
                            &client, &base_url, &token,
                            product_id as i64, &name, None, &path_str,
                        ).await {
                            Ok(_) => {
                                if let Ok(p) = api::products::fetch_one(&client, &base_url, &token, product_id as i64).await {
                                    let _ = db::products::upsert(&pool, &p).await;
                                    if let Some(ref img) = p.image {
                                        let _ = images::ensure(&client, &base_url, &data_dir, img).await;
                                    }
                                    if let Ok(Some(p2)) = db::products::get_by_id(&pool, product_id as i64).await {
                                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                            let full = to_product_full(p2, &data_dir);
                                            ui.set_selected_product(full);
                                        });
                                    }
                                }
                            }
                            Err(e) => eprintln!("[main] change_product_image error: {e}"),
                        }
                    });
                }
            });
        });
    }

    // --- update-product-meta (name + category) ---
    {
        let client_upm = client.clone();
        let rt_upm = rt.clone();
        let base_url_upm = base_url.clone();
        let shared_token_upm = shared_token.clone();
        let pool_upm = pool.clone();
        let data_dir_upm = data_dir.clone();
        let cache_upm = cache.clone();
        let ui_weak = ui.as_weak();

        ui.on_update_product_meta(move |product_id, name, category| {
            let token = match shared_token_upm.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_upm).clone();
            let base_url = base_url_upm.clone();
            let rt = rt_upm.clone();
            let pool = pool_upm.clone();
            let data_dir = data_dir_upm.clone();
            let cache = cache_upm.clone();
            let ui_weak = ui_weak.clone();
            let name = name.to_string();
            let category = category.to_string();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    match api::products::update_product_meta(
                        &client, &base_url, &token,
                        product_id as i64, &name, &category,
                    ).await {
                        Ok(()) => {
                            if let Ok(p) = api::products::fetch_one(&client, &base_url, &token, product_id as i64).await {
                                let _ = db::products::upsert(&pool, &p).await;
                                if let Ok(Some(p2)) = db::products::get_by_id(&pool, product_id as i64).await {
                                    let data_dir2 = data_dir.clone();
                                    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                        let full = crate::to_product_full(p2, &data_dir2);
                                        ui.set_selected_product(full);
                                    });
                                }
                                if let Ok(all) = db::products::all(&pool).await {
                                    sync::refresh_ui(all, cache, data_dir, ui_weak);
                                }
                            }
                        }
                        Err(e) => eprintln!("[main] update_product_meta error: {e}"),
                    }
                });
            });
        });
    }

    // --- delete-product ---
    {
        let client_dp = client.clone();
        let rt_dp = rt.clone();
        let base_url_dp = base_url.clone();
        let shared_token_dp = shared_token.clone();
        let pool_dp = pool.clone();
        let ui_weak = ui.as_weak();

        ui.on_delete_product(move |product_id| {
            let token = match shared_token_dp.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_dp).clone();
            let base_url = base_url_dp.clone();
            let rt = rt_dp.clone();
            let pool = pool_dp.clone();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    match api::products::delete_product(&client, &base_url, &token, product_id as i64).await {
                        Ok(()) => {
                            let _ = db::products::delete(&pool, product_id as i64).await;
                            let _ = ui_weak.upgrade_in_event_loop(|ui| { ui.set_current_view(0); });
                        }
                        Err(e) => eprintln!("[main] delete_product error: {e}"),
                    }
                });
            });
        });
    }

    // --- update-variation ---
    {
        let client_uv = client.clone();
        let rt_uv = rt.clone();
        let base_url_uv = base_url.clone();
        let shared_token_uv = shared_token.clone();
        let pool_uv = pool.clone();
        let data_dir_uv = data_dir.clone();
        let ui_weak = ui.as_weak();

        ui.on_update_variation(move |product_id, variation_id, dims, pack, std_val, price| {
            let token = match shared_token_uv.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_uv).clone();
            let base_url = base_url_uv.clone();
            let rt = rt_uv.clone();
            let pool = pool_uv.clone();
            let data_dir = data_dir_uv.clone();
            let ui_weak = ui_weak.clone();
            let update = api::products::VariationUpdate {
                dimensions: opt_str(&dims),
                packaging: opt_str(&pack),
                standard: opt_str(&std_val),
                price: price as f64,
            };

            std::thread::spawn(move || {
                rt.block_on(async move {
                    if let Err(e) = api::products::update_variation(
                        &client, &base_url, &token,
                        product_id as i64, variation_id as i64, update
                    ).await {
                        eprintln!("[main] update_variation error: {e}");
                        return;
                    }
                    if let Ok(Some(p)) = db::products::get_by_id(&pool, product_id as i64).await {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            let full = to_product_full(p, &data_dir);
                            ui.set_selected_product(full);
                        });
                    }
                });
            });
        });
    }

    // --- delete-variation ---
    {
        let client_dv = client.clone();
        let rt_dv = rt.clone();
        let base_url_dv = base_url.clone();
        let shared_token_dv = shared_token.clone();
        let pool_dv = pool.clone();
        let data_dir_dv = data_dir.clone();
        let ui_weak = ui.as_weak();

        ui.on_delete_variation(move |product_id, variation_id| {
            let token = match shared_token_dv.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_dv).clone();
            let base_url = base_url_dv.clone();
            let rt = rt_dv.clone();
            let pool = pool_dv.clone();
            let data_dir = data_dir_dv.clone();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    if let Err(e) = api::products::delete_variation(
                        &client, &base_url, &token,
                        product_id as i64, variation_id as i64
                    ).await {
                        eprintln!("[main] delete_variation error: {e}");
                        return;
                    }
                    if let Ok(Some(p)) = db::products::get_by_id(&pool, product_id as i64).await {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            let full = to_product_full(p, &data_dir);
                            ui.set_selected_product(full);
                        });
                    }
                });
            });
        });
    }

    // --- add-variation ---
    {
        let client_av = client.clone();
        let rt_av = rt.clone();
        let base_url_av = base_url.clone();
        let shared_token_av = shared_token.clone();
        let pool_av = pool.clone();
        let data_dir_av = data_dir.clone();
        let ui_weak = ui.as_weak();

        ui.on_add_variation(move |product_id, dims, pack, std_val, price| {
            let token = match shared_token_av.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_av).clone();
            let base_url = base_url_av.clone();
            let rt = rt_av.clone();
            let pool = pool_av.clone();
            let data_dir = data_dir_av.clone();
            let ui_weak = ui_weak.clone();
            let variation = api::products::NewVariation {
                dimensions: opt_str(&dims),
                packaging: opt_str(&pack),
                standard: opt_str(&std_val),
                price: price as f64,
            };

            std::thread::spawn(move || {
                rt.block_on(async move {
                    if let Err(e) = api::products::add_variation(
                        &client, &base_url, &token,
                        product_id as i64, variation
                    ).await {
                        eprintln!("[main] add_variation error: {e}");
                        return;
                    }
                    match api::products::fetch_one(&client, &base_url, &token, product_id as i64).await {
                        Ok(p) => {
                            let _ = db::products::upsert(&pool, &p).await;
                            if let Ok(Some(p2)) = db::products::get_by_id(&pool, product_id as i64).await {
                                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                    let full = to_product_full(p2, &data_dir);
                                    ui.set_selected_product(full);
                                });
                            }
                        }
                        Err(e) => eprintln!("[main] fetch_one after add_variation: {e}"),
                    }
                });
            });
        });
    }

    // --- refresh ---
    {
        let client_r = client.clone();
        let rt_r = rt.clone();
        let base_url_r = base_url.clone();
        let shared_token_r = shared_token.clone();
        let pool_r = pool.clone();
        let data_dir_r = data_dir.clone();
        let cache_r = cache.clone();
        let ui_weak = ui.as_weak();

        ui.on_refresh(move || {
            let token = match shared_token_r.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_r).clone();
            let base_url = base_url_r.clone();
            let rt = rt_r.clone();
            let pool = pool_r.clone();
            let data_dir = data_dir_r.clone();
            let cache = cache_r.clone();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    if let Err(e) = sync::initial_sync(
                        &client, &base_url, &token, &pool, &data_dir,
                        cache, ui_weak,
                    ).await {
                        eprintln!("[main] refresh error: {e}");
                    }
                });
            });
        });
    }

    // --- load-orders ---
    {
        let client_lo = client.clone();
        let rt_lo = rt.clone();
        let base_url_lo = base_url.clone();
        let shared_token_lo = shared_token.clone();
        let pool_lo = pool.clone();
        let ui_weak = ui.as_weak();

        struct ItemDisplay {
            variation_id: i64,
            product_name: String,
            quantity: i64,
            discount: f64,
            price: f64,
            dimensions: String,
            packaging: String,
            standard: String,
        }
        struct OrderDisplay {
            id: i64,
            client_name: String,
            address_invoice: String,
            address_delivery: String,
            total: f64,
            discount: f64,
            created_at: String,
            items: Vec<ItemDisplay>,
        }

        ui.on_load_orders(move || {
            let token = match shared_token_lo.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_lo).clone();
            let base_url = base_url_lo.clone();
            let rt = rt_lo.clone();
            let pool = pool_lo.clone();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    match api::orders::fetch_all(&client, &base_url, &token).await {
                        Ok(list) => {
                            let mut display_orders: Vec<OrderDisplay> = Vec::new();
                            for o in &list {
                                let full = match api::orders::fetch_one(&client, &base_url, &token, o.id).await {
                                    Ok(f) => f,
                                    Err(e) => {
                                        eprintln!("[main] fetch_one order {} error: {e}", o.id);
                                        o.clone()
                                    }
                                };

                                let mut items: Vec<ItemDisplay> = Vec::new();
                                for it in &full.items {
                                    let (product_name, price, dimensions, packaging, standard) =
                                        if let Some(v) = &it.variation {
                                            let pname = match db::products::get_variation_by_id(&pool, it.variation_id).await {
                                                Ok(Some(row)) => row.product_name,
                                                _ => String::new(),
                                            };
                                            (
                                                pname,
                                                v.price,
                                                v.dimensions.clone().unwrap_or_default(),
                                                v.packaging.clone().unwrap_or_default(),
                                                v.standard.clone().unwrap_or_default(),
                                            )
                                        } else {
                                            match db::products::get_variation_by_id(&pool, it.variation_id).await {
                                                Ok(Some(row)) => (
                                                    row.product_name,
                                                    row.price,
                                                    row.dimensions.unwrap_or_default(),
                                                    row.packaging.unwrap_or_default(),
                                                    row.standard.unwrap_or_default(),
                                                ),
                                                Ok(None) => {
                                                    eprintln!("[main] variation {} not in local DB", it.variation_id);
                                                    (String::new(), 0.0, String::new(), String::new(), String::new())
                                                }
                                                Err(e) => {
                                                    eprintln!("[main] get_variation_by_id error: {e}");
                                                    (String::new(), 0.0, String::new(), String::new(), String::new())
                                                }
                                            }
                                        };

                                    items.push(ItemDisplay {
                                        variation_id: it.variation_id,
                                        product_name,
                                        quantity: it.quantity,
                                        discount: it.discount.unwrap_or(0.0),
                                        price,
                                        dimensions,
                                        packaging,
                                        standard,
                                    });
                                }

                                display_orders.push(OrderDisplay {
                                    id: full.id,
                                    client_name: full.client_name.clone(),
                                    address_invoice: full.address_invoice.clone().unwrap_or_default(),
                                    address_delivery: full.address_delivery.clone().unwrap_or_default(),
                                    total: full.total,
                                    discount: full.discount.unwrap_or(0.0),
                                    created_at: full.created_at.clone(),
                                    items,
                                });
                            }

                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                let slint_orders: Vec<OrderData> = display_orders.into_iter().map(|o| {
                                    let items: Vec<OrderItem> = o.items.into_iter().map(|it| OrderItem {
                                        variation_id: it.variation_id as i32,
                                        product_name: it.product_name.into(),
                                        quantity: it.quantity as i32,
                                        item_discount: it.discount as f32,
                                        price: it.price as f32,
                                        dimensions: it.dimensions.into(),
                                        packaging: it.packaging.into(),
                                        standard: it.standard.into(),
                                    }).collect();
                                    OrderData {
                                        id: o.id as i32,
                                        client_name: o.client_name.into(),
                                        address_invoice: o.address_invoice.into(),
                                        address_delivery: o.address_delivery.into(),
                                        total: o.total as f32,
                                        discount: o.discount as f32,
                                        created_at: o.created_at.into(),
                                        items: Rc::new(slint::VecModel::from(items)).into(),
                                    }
                                }).collect();
                                ui.set_orders(Rc::new(slint::VecModel::from(slint_orders)).into());
                            });
                        }
                        Err(e) => eprintln!("[main] load_orders error: {e}"),
                    }
                });
            });
        });
    }

    // --- load-users ---
    {
        let client_lu = client.clone();
        let rt_lu = rt.clone();
        let base_url_lu = base_url.clone();
        let shared_token_lu = shared_token.clone();
        let ui_weak = ui.as_weak();

        ui.on_load_users(move || {
            let token = match shared_token_lu.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_lu).clone();
            let base_url = base_url_lu.clone();
            let rt = rt_lu.clone();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    match api::users::fetch_all(&client, &base_url, &token).await {
                        Ok(users) => {
                            let slint_users: Vec<UserData> = users.iter().map(|u| UserData {
                                id: u.id as i32,
                                name: u.name.clone().into(),
                                email: u.email.clone().into(),
                                is_active: u.is_active,
                                is_superuser: u.is_superuser,
                            }).collect();
                            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                                ui.set_users(Rc::new(slint::VecModel::from(slint_users)).into());
                            });
                        }
                        Err(e) => eprintln!("[main] load_users error: {e}"),
                    }
                });
            });
        });
    }

    // --- create-user ---
    {
        let client_cu = client.clone();
        let rt_cu = rt.clone();
        let base_url_cu = base_url.clone();
        let shared_token_cu = shared_token.clone();
        let ui_weak = ui.as_weak();

        ui.on_create_user(move |username, email, password, is_active, is_superuser| {
            let token = match shared_token_cu.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_cu).clone();
            let base_url = base_url_cu.clone();
            let rt = rt_cu.clone();
            let ui_weak = ui_weak.clone();
            let user = api::users::CreateUser {
                name: username.to_string(),
                email: email.to_string(),
                password: password.to_string(),
                is_active,
                is_superuser,
            };

            std::thread::spawn(move || {
                rt.block_on(async move {
                    if let Err(e) = api::users::create(&client, &base_url, &token, user).await {
                        eprintln!("[main] create_user error: {e}");
                        return;
                    }
                    if let Ok(users) = api::users::fetch_all(&client, &base_url, &token).await {
                        let slint_users: Vec<UserData> = users.iter().map(|u| UserData {
                            id: u.id as i32,
                            name: u.name.clone().into(),
                            email: u.email.clone().into(),
                            is_active: u.is_active,
                            is_superuser: u.is_superuser,
                        }).collect();
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_users(Rc::new(slint::VecModel::from(slint_users)).into());
                        });
                    }
                });
            });
        });
    }

    // --- update-user ---
    {
        let client_uu = client.clone();
        let rt_uu = rt.clone();
        let base_url_uu = base_url.clone();
        let shared_token_uu = shared_token.clone();
        let ui_weak = ui.as_weak();

        ui.on_update_user(move |user_id, username, email, is_active, is_superuser| {
            let token = match shared_token_uu.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_uu).clone();
            let base_url = base_url_uu.clone();
            let rt = rt_uu.clone();
            let ui_weak = ui_weak.clone();
            let user = api::users::UpdateUser {
                name: username.to_string(),
                email: email.to_string(),
                is_active,
                is_superuser,
            };

            std::thread::spawn(move || {
                rt.block_on(async move {
                    if let Err(e) = api::users::update(&client, &base_url, &token, user_id as i64, user).await {
                        eprintln!("[main] update_user error: {e}");
                        return;
                    }
                    if let Ok(users) = api::users::fetch_all(&client, &base_url, &token).await {
                        let slint_users: Vec<UserData> = users.iter().map(|u| UserData {
                            id: u.id as i32,
                            name: u.name.clone().into(),
                            email: u.email.clone().into(),
                            is_active: u.is_active,
                            is_superuser: u.is_superuser,
                        }).collect();
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_users(Rc::new(slint::VecModel::from(slint_users)).into());
                        });
                    }
                });
            });
        });
    }

    // --- delete-user ---
    {
        let client_du = client.clone();
        let rt_du = rt.clone();
        let base_url_du = base_url.clone();
        let shared_token_du = shared_token.clone();
        let ui_weak = ui.as_weak();

        ui.on_delete_user(move |user_id| {
            let token = match shared_token_du.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_du).clone();
            let base_url = base_url_du.clone();
            let rt = rt_du.clone();
            let ui_weak = ui_weak.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    if let Err(e) = api::users::delete(&client, &base_url, &token, user_id as i64).await {
                        eprintln!("[main] delete_user error: {e}");
                        return;
                    }
                    if let Ok(users) = api::users::fetch_all(&client, &base_url, &token).await {
                        let slint_users: Vec<UserData> = users.iter().map(|u| UserData {
                            id: u.id as i32,
                            name: u.name.clone().into(),
                            email: u.email.clone().into(),
                            is_active: u.is_active,
                            is_superuser: u.is_superuser,
                        }).collect();
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_users(Rc::new(slint::VecModel::from(slint_users)).into());
                        });
                    }
                });
            });
        });
    }

    // --- toggle-include-in-catalog ---
    {
        let client_tic = client.clone();
        let rt_tic = rt.clone();
        let base_url_tic = base_url.clone();
        let shared_token_tic = shared_token.clone();
        let cache_tic = cache.clone();
        let pool_tic = pool.clone();
        let ui_handle_tic = ui.as_weak();

        ui.on_toggle_include_in_catalog(move |product_id, include| {
            let token = match shared_token_tic.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_tic).clone();
            let base_url = base_url_tic.clone();
            let rt = rt_tic.clone();
            let cache = cache_tic.clone();
            let pool = pool_tic.clone();
            let ui_handle = ui_handle_tic.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    match api::products::toggle_include_in_catalog(&client, &base_url, &token, product_id as i64, include).await {
                        Ok(_) => {
                            // Fetch updated product and update local DB + cache
                            match api::products::fetch_one(&client, &base_url, &token, product_id as i64).await {
                                Ok(p) => {
                                    let data_dir = config::Config::data_dir();
                                    let _ = db::products::upsert(&pool, &p).await;
                                    {
                                        let mut c = cache.lock().unwrap();
                                        if let Some(pos) = c.iter().position(|x: &models::product::ProductData| x.id == p.id) {
                                            c[pos] = p.clone();
                                        }
                                    }
                                    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                                        let full = to_product_full(p, &data_dir);
                                        ui.set_selected_product(full);
                                    });
                                }
                                Err(e) => eprintln!("[main] toggle_include_in_catalog fetch_one error: {e}"),
                            }
                        }
                        Err(e) => eprintln!("[main] toggle_include_in_catalog error: {e}"),
                    }
                });
            });
        });
    }

    // --- download-catalog-pdf ---
    {
        let client_pdf = client.clone();
        let rt_pdf = rt.clone();
        let base_url_pdf = base_url.clone();
        let shared_token_pdf = shared_token.clone();

        ui.on_download_catalog_pdf(move || {
            let token = match shared_token_pdf.lock().unwrap().clone() {
                Some(t) => t,
                None => return,
            };
            let client = (*client_pdf).clone();
            let base_url = base_url_pdf.clone();
            let rt = rt_pdf.clone();

            std::thread::spawn(move || {
                rt.block_on(async move {
                    match api::products::download_catalog_pdf(&client, &base_url, &token, 3, "price", false, "default").await {
                        Ok(bytes) => {
                            let downloads = std::env::var("HOME")
                                .or_else(|_| std::env::var("USERPROFILE"))
                                .map(|h| std::path::PathBuf::from(h).join("Downloads"))
                                .unwrap_or_else(|_| std::path::PathBuf::from("."));
                            let path = downloads.join("catalog.pdf");
                            match tokio::fs::write(&path, &bytes).await {
                                Ok(_) => eprintln!("[main] catalog PDF saved to {}", path.display()),
                                Err(e) => eprintln!("[main] failed to save PDF: {e}"),
                            }
                        }
                        Err(e) => eprintln!("[main] download_catalog_pdf error: {e}"),
                    }
                });
            });
        });
    }

    ui.run()
}

#[cfg(target_os = "android")]
pub static ANDROID_DATA_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    if let Some(path) = app.internal_data_path() {
        let _ = ANDROID_DATA_DIR.set(path);
    }
    slint::android::init(app).unwrap();
    run_app().unwrap();
}
