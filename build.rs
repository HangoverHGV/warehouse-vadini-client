fn main() {
    slint_build::compile("ui/main.slint").expect("Slint build failed");

    println!("cargo:rerun-if-changed=Logo.png");
    println!("cargo:rerun-if-changed=config.json");

    // Copy config.json next to the binary automatically
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let binary_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap();
    let dest = binary_dir.join("config.json");
    if !dest.exists() {
        std::fs::copy("config.json", &dest)
            .expect("Failed to copy config.json to build directory");
    }

    // Windows: generate Logo.ico from Logo.png and embed as the app icon
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let ico_path = std::path::PathBuf::from(&out_dir).join("Logo.ico");
        let img = image::open("Logo.png").expect("Failed to open Logo.png");
        img.resize(256, 256, image::imageops::FilterType::Lanczos3)
            .save(&ico_path)
            .expect("Failed to write Logo.ico");

        let mut res = winres::WindowsResource::new();
        res.set_icon(ico_path.to_str().unwrap());
        res.compile().expect("Failed to embed Windows icon");
    }
}
