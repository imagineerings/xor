# Dependency reconciliation audit

Upstream authority: `eb8e1c8b5502b7007465fbbc465f4a736fa39210` (`v1.16.1`).

The comparison covers dependency, development-dependency, build-dependency,
target dependency, and Cargo patch declarations in every manifest present at
v1.16.1. Package repository metadata is not a dependency declaration.

| Result | Count |
| --- | ---: |
| Exact upstream declarations | 5717 |
| Drifted upstream declarations | 0 |
| Missing upstream declarations | 0 |
| Missing upstream manifests | 0 |
| Sim additions in existing manifests | 79 |
| New Sim manifests | 27 |
| Unapproved Sim fork declarations | 0 |
| Preserved v1.16.1 external lock records | 1608 |
| Missing/replaced v1.16.1 external lock records | 3 |
| Added external lock records | 28 |
| Unapproved Sim fork lock records | 0 |

## Drifted upstream declarations

None.

## Resolved lockfile drift

### Missing or replaced v1.16.1 external records

- `cap-primitives` `3.4.4` — `registry+https://github.com/rust-lang/crates.io-index`
- `cap-std` `3.4.4` — `registry+https://github.com/rust-lang/crates.io-index`
- `flate2` `1.1.8` — `registry+https://github.com/rust-lang/crates.io-index`

### Added external records

- `aead` `0.5.2` — `registry+https://github.com/rust-lang/crates.io-index`
- `atomic-polyfill` `1.0.3` — `registry+https://github.com/rust-lang/crates.io-index`
- `bech32` `0.11.1` — `registry+https://github.com/rust-lang/crates.io-index`
- `bip39` `2.2.2` — `registry+https://github.com/rust-lang/crates.io-index`
- `bitcoin-consensus-encoding` `1.2.0` — `registry+https://github.com/rust-lang/crates.io-index`
- `bitcoin-internals` `0.6.0` — `registry+https://github.com/rust-lang/crates.io-index`
- `bitcoin-io` `0.1.101` — `registry+https://github.com/rust-lang/crates.io-index`
- `bitcoin_hashes` `0.14.101` — `registry+https://github.com/rust-lang/crates.io-index`
- `cap-primitives` `3.4.6` — `registry+https://github.com/rust-lang/crates.io-index`
- `cap-std` `3.4.6` — `registry+https://github.com/rust-lang/crates.io-index`
- `chacha20` `0.9.1` — `registry+https://github.com/rust-lang/crates.io-index`
- `chacha20poly1305` `0.10.1` — `registry+https://github.com/rust-lang/crates.io-index`
- `critical-section` `1.2.0` — `registry+https://github.com/rust-lang/crates.io-index`
- `flate2` `1.1.9` — `registry+https://github.com/rust-lang/crates.io-index`
- `hash32` `0.2.1` — `registry+https://github.com/rust-lang/crates.io-index`
- `heapless` `0.7.17` — `registry+https://github.com/rust-lang/crates.io-index`
- `hex-conservative` `0.2.2` — `registry+https://github.com/rust-lang/crates.io-index`
- `hex-conservative` `1.2.0` — `registry+https://github.com/rust-lang/crates.io-index`
- `nostr` `0.44.7` — `registry+https://github.com/rust-lang/crates.io-index`
- `opaque-debug` `0.3.1` — `registry+https://github.com/rust-lang/crates.io-index`
- `password-hash` `0.5.0` — `registry+https://github.com/rust-lang/crates.io-index`
- `poly1305` `0.8.0` — `registry+https://github.com/rust-lang/crates.io-index`
- `pp-rs` `0.2.1` — `registry+https://github.com/rust-lang/crates.io-index`
- `salsa20` `0.10.2` — `registry+https://github.com/rust-lang/crates.io-index`
- `scrypt` `0.11.0` — `registry+https://github.com/rust-lang/crates.io-index`
- `secp256k1` `0.29.1` — `registry+https://github.com/rust-lang/crates.io-index`
- `secp256k1-sys` `0.10.1` — `registry+https://github.com/rust-lang/crates.io-index`
- `universal-hash` `0.5.1` — `registry+https://github.com/rust-lang/crates.io-index`

### Unapproved Sim fork lock records

None.

## Missing upstream declarations

None.

## Unapproved Sim fork declarations

None.

## Sim additions

New declarations are retained for review because absence from the upstream
manifest is not itself evidence of an upstream dependency substitution.

- `Cargo.toml` / `workspace > dependencies > bech32`: `"0.11"`
- `Cargo.toml` / `workspace > dependencies > bytemuck`: `{"features":["derive"],"version":"1.23"}`
- `Cargo.toml` / `workspace > dependencies > cap-std`: `"3.4.5"`
- `Cargo.toml` / `workspace > dependencies > cargo_ui`: `{"path":"crates/cargo_ui"}`
- `Cargo.toml` / `workspace > dependencies > collaboration_domain`: `{"path":"crates/collaboration_domain"}`
- `Cargo.toml` / `workspace > dependencies > comfy_api`: `{"path":"crates/comfy_api"}`
- `Cargo.toml` / `workspace > dependencies > comfy_backend_corex`: `{"path":"crates/comfy_backend_corex"}`
- `Cargo.toml` / `workspace > dependencies > comfy_backend_cuda`: `{"path":"crates/comfy_backend_cuda"}`
- `Cargo.toml` / `workspace > dependencies > comfy_backend_directml`: `{"path":"crates/comfy_backend_directml"}`
- `Cargo.toml` / `workspace > dependencies > comfy_backend_metal`: `{"path":"crates/comfy_backend_metal"}`
- `Cargo.toml` / `workspace > dependencies > comfy_backend_mlu`: `{"path":"crates/comfy_backend_mlu"}`
- `Cargo.toml` / `workspace > dependencies > comfy_backend_npu`: `{"path":"crates/comfy_backend_npu"}`
- `Cargo.toml` / `workspace > dependencies > comfy_backend_rocm`: `{"path":"crates/comfy_backend_rocm"}`
- `Cargo.toml` / `workspace > dependencies > comfy_backend_xpu`: `{"path":"crates/comfy_backend_xpu"}`
- `Cargo.toml` / `workspace > dependencies > comfy_media`: `{"path":"crates/comfy_media"}`
- `Cargo.toml` / `workspace > dependencies > comfy_model`: `{"path":"crates/comfy_model"}`
- `Cargo.toml` / `workspace > dependencies > comfy_nodes`: `{"path":"crates/comfy_nodes"}`
- `Cargo.toml` / `workspace > dependencies > comfy_plugin_host`: `{"path":"crates/comfy_plugin_host"}`
- `Cargo.toml` / `workspace > dependencies > comfy_plugin_sdk`: `{"path":"crates/comfy_plugin_sdk"}`
- `Cargo.toml` / `workspace > dependencies > comfy_runtime`: `{"path":"crates/comfy_runtime"}`
- `Cargo.toml` / `workspace > dependencies > comfy_sampler`: `{"path":"crates/comfy_sampler"}`
- `Cargo.toml` / `workspace > dependencies > comfy_tensor`: `{"path":"crates/comfy_tensor"}`
- `Cargo.toml` / `workspace > dependencies > comfy_test_support`: `{"path":"crates/comfy_test_support"}`
- `Cargo.toml` / `workspace > dependencies > comfy_types`: `{"path":"crates/comfy_types"}`
- `Cargo.toml` / `workspace > dependencies > comfy_ui`: `{"path":"crates/comfy_ui"}`
- `Cargo.toml` / `workspace > dependencies > comfy_worker`: `{"path":"crates/comfy_worker"}`
- `Cargo.toml` / `workspace > dependencies > flate2`: `"1.1.9"`
- `Cargo.toml` / `workspace > dependencies > getrandom`: `"0.3.4"`
- `Cargo.toml` / `workspace > dependencies > half`: `{"features":["bytemuck","serde"],"version":"2.6"}`
- `Cargo.toml` / `workspace > dependencies > naga`: `{"features":["glsl-in"],"version":"29.0.4"}`
- `Cargo.toml` / `workspace > dependencies > nostr`: `{"default-features":false,"features":["std","nip49"],"version":"=0.44.7"}`
- `Cargo.toml` / `workspace > dependencies > nostr_compat`: `{"path":"crates/nostr_compat"}`
- `Cargo.toml` / `workspace > dependencies > postcard`: `{"features":["alloc","use-std"],"version":"1.1"}`
- `Cargo.toml` / `workspace > dependencies > ring`: `"0.17.14"`
- `Cargo.toml` / `workspace > dependencies > secp256k1`: `{"default-features":false,"features":["std"],"version":"0.29.1"}`
- `Cargo.toml` / `workspace > dependencies > unicode-normalization`: `"0.1"`
- `crates/agent_ui/Cargo.toml` / `dependencies > git_ui`: `{"workspace":true}`
- `crates/channel/Cargo.toml` / `dependencies > collaboration_domain`: `{"optional":true,"workspace":true}`
- `crates/channel/Cargo.toml` / `dev-dependencies > uuid`: `{"workspace":true}`
- `crates/collab/Cargo.toml` / `dependencies > base64`: `{"workspace":true}`
- `crates/collab/Cargo.toml` / `dependencies > collaboration_domain`: `{"workspace":true}`
- `crates/collab/Cargo.toml` / `dependencies > nostr_compat`: `{"workspace":true}`
- `crates/collab/Cargo.toml` / `dependencies > thiserror`: `{"workspace":true}`
- `crates/collab/Cargo.toml` / `dependencies > url`: `{"workspace":true}`
- `crates/collab/Cargo.toml` / `dev-dependencies > secp256k1`: `{"workspace":true}`
- `crates/onboarding/Cargo.toml` / `dev-dependencies > settings`: `{"features":["test-support"],"workspace":true}`
- `crates/project/Cargo.toml` / `dependencies > cargo_metadata`: `{"optional":true,"workspace":true}`
- `crates/sidebar/Cargo.toml` / `dependencies > channel`: `{"workspace":true}`
- `crates/sidebar/Cargo.toml` / `dev-dependencies > rpc`: `{"workspace":true}`
- `crates/sidebar/Cargo.toml` / `dev-dependencies > sha2`: `{"workspace":true}`
- `crates/tasks_ui/Cargo.toml` / `dependencies > db`: `{"optional":true,"workspace":true}`
- `crates/tasks_ui/Cargo.toml` / `dependencies > fs`: `{"optional":true,"workspace":true}`
- `crates/tasks_ui/Cargo.toml` / `dependencies > futures`: `{"optional":true,"workspace":true}`
- `crates/tasks_ui/Cargo.toml` / `dependencies > language_tools`: `{"optional":true,"workspace":true}`
- `crates/tasks_ui/Cargo.toml` / `dependencies > log`: `{"optional":true,"workspace":true}`
- `crates/tasks_ui/Cargo.toml` / `dependencies > serde_json`: `{"optional":true,"workspace":true}`
- `crates/tasks_ui/Cargo.toml` / `dependencies > settings`: `{"optional":true,"workspace":true}`
- `crates/tasks_ui/Cargo.toml` / `dependencies > ui_input`: `{"optional":true,"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > async-channel`: `{"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > cargo_ui`: `{"optional":true,"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > comfy_api`: `{"optional":true,"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > comfy_model`: `{"optional":true,"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > comfy_plugin_host`: `{"optional":true,"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > comfy_runtime`: `{"optional":true,"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > comfy_tensor`: `{"optional":true,"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > comfy_types`: `{"optional":true,"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > comfy_ui`: `{"optional":true,"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > sha2`: `{"workspace":true}`
- `crates/zed/Cargo.toml` / `dependencies > thiserror`: `{"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > bech32`: `{"optional":true,"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > collaboration_domain`: `{"optional":true,"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > getrandom`: `{"optional":true,"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > hex`: `{"optional":true,"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > nostr`: `{"optional":true,"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > secp256k1`: `{"optional":true,"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > sha2`: `{"optional":true,"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > thiserror`: `{"optional":true,"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > uuid`: `{"optional":true,"workspace":true}`
- `crates/zed_credentials_provider/Cargo.toml` / `dependencies > zeroize`: `{"optional":true,"workspace":true}`

## New Sim manifests

- `crates/cargo_ui/Cargo.toml`
- `crates/collaboration_domain/Cargo.toml`
- `crates/comfy_api/Cargo.toml`
- `crates/comfy_backend_corex/Cargo.toml`
- `crates/comfy_backend_cuda/Cargo.toml`
- `crates/comfy_backend_directml/Cargo.toml`
- `crates/comfy_backend_metal/Cargo.toml`
- `crates/comfy_backend_mlu/Cargo.toml`
- `crates/comfy_backend_npu/Cargo.toml`
- `crates/comfy_backend_rocm/Cargo.toml`
- `crates/comfy_backend_xpu/Cargo.toml`
- `crates/comfy_media/Cargo.toml`
- `crates/comfy_model/Cargo.toml`
- `crates/comfy_nodes/Cargo.toml`
- `crates/comfy_plugin_host/Cargo.toml`
- `crates/comfy_plugin_host/tests/fixtures/hang_component_source/Cargo.toml`
- `crates/comfy_plugin_host/tests/fixtures/list_ports_component/Cargo.toml`
- `crates/comfy_plugin_host/tests/fixtures/provider_component_source/Cargo.toml`
- `crates/comfy_plugin_sdk/Cargo.toml`
- `crates/comfy_runtime/Cargo.toml`
- `crates/comfy_sampler/Cargo.toml`
- `crates/comfy_tensor/Cargo.toml`
- `crates/comfy_test_support/Cargo.toml`
- `crates/comfy_types/Cargo.toml`
- `crates/comfy_ui/Cargo.toml`
- `crates/comfy_worker/Cargo.toml`
- `crates/nostr_compat/Cargo.toml`
