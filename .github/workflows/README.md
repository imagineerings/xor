# Validation scope

The required `run_tests` workflow validates the enabled Rust product rather than every test target in the repository. Its `rustlings_tests` job is generated from the Rust product manifest features and an explicit package list in `tooling/xtask/src/tasks/workflows/run_tests.rs`.

The required desktop collaboration baseline includes `client`, `call`, `channel`, and `collab_ui` without `multiplayer-tools`. These packages provide application connectivity, calls, channels, and collaboration UI used by the Rust product even when its multiplayer workspace is not compiled.

## Deferred suites

Deferred suites are not counted as passing when they do not run.

### Multiplayer desktop behavior

`multiplayer-tools` behavior is outside the current Rust product because `products/flavors.toml` enables exactly `agentic-tools,rust-tools`. When that product enables `multiplayer-tools`, add its desktop tests to the required product gate using:

```sh
ZED_PRODUCT_ID=rust cargo nextest run --no-default-features \
  --features zed/agentic-tools,zed/rust-tools,zed/multiplayer-tools,channel/multiplayer-tools,collab_ui/multiplayer-tools \
  -p zed -p channel -p collab_ui
```

### Hosted Collab server

The `collab` package is the hosted collaboration server. Its `collab_tests` integration target requires the `test-support` feature and PostgreSQL. The generated `hosted_collab_tests` workflow provides PostgreSQL 15, sets `USE_POSTGRES=true` so the shared integration server uses it, and runs manually or whenever hosted collaboration paths change:

```sh
cargo nextest run --package collab --features test-support --test collab_tests \
  --no-fail-fast --no-tests=warn
```

This workflow becomes a required product gate if Rustlings enables `multiplayer-tools`, operates its own Collab service, or otherwise makes hosted collaboration part of the shipped product. It remains a separate PostgreSQL-backed gate rather than being mixed into lightweight desktop product tests.

### Comfy runtime and evidence

The Linux, macOS, and Windows Comfy backend warning-denied Clippy matrix remains required. Comfy runtime and evidence tests are deferred until Comfy becomes a shipped product feature and CI has the pinned source/evidence fixtures plus the filesystem and runtime resources those suites certify. Restore them as a dedicated job with:

```sh
cargo nextest run --no-fail-fast --no-tests=warn \
  -p comfy_api -p comfy_media -p comfy_model -p comfy_nodes \
  -p comfy_plugin_host -p comfy_plugin_sdk -p comfy_plugin_worker_fixture \
  -p comfy_runtime -p comfy_sampler -p comfy_tensor -p comfy_test_support \
  -p comfy_types -p comfy_ui -p comfy_worker
```

The dedicated job must provision the repository-pinned Comfy source/evidence fixture trees, filesystem space, and any certified runtime resources before this command becomes required.
