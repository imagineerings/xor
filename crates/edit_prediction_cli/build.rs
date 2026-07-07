fn main() {
    let cargo_toml = std::fs::read_to_string("../sim/Cargo.toml")
        .expect("Failed to read crates/sim/Cargo.toml");
    let version = cargo_toml
        .lines()
        .find(|line| line.starts_with("version = "))
        .expect("Version not found in crates/sim/Cargo.toml")
        .split('=')
        .nth(1)
        .expect("Invalid version format")
        .trim()
        .trim_matches('"');
    println!("cargo:rustc-env=SIM_PKG_VERSION={}", version);
}
