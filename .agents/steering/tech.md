# Tech Stack

## Language & Toolchain
- **Rust** (pinned to `1.95.0` via `rust-toolchain.toml`)
- Edition: 2024
- Cargo workspace resolver v2

## UI Framework
- **GPUI** — Zed's own GPU-accelerated UI framework, lives in `crates/gpui`. Flexbox layout, Metal/wgpu rendering, single foreground thread for all UI/entity updates.

## Key Libraries
- **Tree-sitter** — incremental syntax parsing for all languages
- **WASM / Wasmtime** — extension host runs extensions as WASM components
- **LiveKit** — real-time audio/video for collaboration
- **alacritty_terminal** — terminal emulation
- **SQLite (sqlez)** — local DB via a custom async SQLite wrapper
- **smol** + **tokio** (via gpui_tokio) — async runtimes; smol is primary, tokio for some dependencies
- **proptest** — property-based testing
- **cargo-nextest** — preferred test runner (configured in `.config/nextest.toml`)
- **serde / serde_json** — serialization throughout
- **anyhow / thiserror** — error handling

## LLM Provider Integrations
Separate crates per provider: `anthropic`, `open_ai`, `google_ai`, `bedrock`, `ollama`, `deepseek`, `mistral`, `codestral`, `lmstudio`, `x_ai`, `open_router`, `cloud_llm_client`

## Build Targets
- `wasm32-wasip2` — extensions
- `wasm32-unknown-unknown` — GPUI on the web
- `x86_64-unknown-linux-musl` — remote server

## Common Commands

```sh
# Run Zed (debug build)
cargo run

# Run Zed (release build)
cargo run --release

# Run all tests
cargo test --workspace

# Run tests with nextest (preferred in CI)
cargo nextest run --workspace

# Lint — use the script, not cargo clippy directly
./script/clippy

# Format
cargo fmt

# Run a local collab server + multiple Zed instances for collaboration testing
./script/zed-local
./script/zed-local -2   # two instances

# Bootstrap dev environment (macOS)
./script/bootstrap

# Bundle macOS app
./script/bundle-mac

# Create a new crate
./script/new-crate <name>

# Fetch a Sentry crash report
./script/sentry-fetch <issue-id>

# Generate an investigation prompt from a crash
./script/crash-to-prompt <issue-id>
```

## CI / Config
- `.cargo/config.toml` — sets `symbol-mangling-version=v0`, `tokio_unstable`, and platform-specific flags
- `.config/nextest.toml` — slow test timeouts, sequential DB tests, priority ordering
- `clippy.toml` — clippy configuration
- `rustfmt.toml` — formatting rules
- `typos.toml` — spell-checking config
- `renovate.json` — dependency update automation
