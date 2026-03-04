fn main() {
    slint_build::compile("ui/main.slint").expect("Slint build failed");

    // Copy config.json next to the binary automatically
    println!("cargo:rerun-if-changed=config.json");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let binary_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap();
    std::fs::copy("config.json", binary_dir.join("config.json"))
        .expect("Failed to copy config.json to build directory");
}