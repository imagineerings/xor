fn main() {
    if let Ok(bundled) = std::env::var("SIM_BUNDLE") {
        println!("cargo:rustc-env=SIM_BUNDLE={}", bundled);
    }
}
