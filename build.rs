fn main() {
    slint_build::compile("ui/main.slint").expect("Slint build failed");

    // Copy config.json next to the binary automatically
    println!("cargo:rerun-if-changed=config.json");
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
}