fn main() {
    if let Ok(bundled) = std::env::var("BAYMAX_BUNDLE") {
        println!("cargo:rustc-env=BAYMAX_BUNDLE={}", bundled);
    }
}
