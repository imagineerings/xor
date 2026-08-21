# Asset reconciliation audit

## Authority

- Upstream asset tree: `v1.16.1` at `eb8e1c8b5502b7007465fbbc465f4a736fa39210`
- Scope: every file under `assets/`, plus `IconName`, `VectorName`, and affected Rust call sites
- Reconciliation rule: existing upstream paths, contents, enum identities, and references remain authoritative; Sim branding is additive work outside this port correction

## Existing upstream assets

The v1.16.1 tree contains 463 asset files. The current tree contains all 463 at
the same paths with Git blob IDs matching v1.16.1. There are zero missing,
renamed, or content-drifted upstream assets.

This includes restoration of:

- `assets/icons/ai_zed.svg`
- `assets/icons/zed_agent.svg` and `assets/icons/zed_agent_two.svg`
- `assets/icons/zed_assistant.svg`
- `assets/icons/zed_predict.svg` and its disabled/down/error/up variants
- `assets/icons/zed_src_custom.svg` and `assets/icons/zed_src_extension.svg`
- `assets/images/zed_logo.svg` and `assets/images/zed_x_copilot.svg`
- the upstream default Linux, macOS, Windows, and Sublime Text keymaps
- `assets/settings/default.json`
- the upstream Ayu, Gruvbox, and One theme metadata

## Restored code identities

The following v1.16.1 variants and all Rust call sites are restored:

- `AiZed`
- `ZedAgent`, `ZedAgentTwo`, and `ZedAssistant`
- `ZedPredict`, `ZedPredictDisabled`, `ZedPredictDown`, `ZedPredictError`, and `ZedPredictUp`
- `ZedSrcCustom` and `ZedSrcExtension`
- `VectorName::ZedLogo` and `VectorName::ZedXCopilot`

The `ZedPredictModal` key context is also restored so the exact upstream default
keymaps match the runtime context. The exhaustive icon tests remain unchanged
from v1.16.1, and the vector tests retain the useful bidirectional inventory
coverage added during diagnosis while validating the upstream names.

## Genuinely new Sim assets retained

Only two files are absent from the v1.16.1 asset tree:

| Path | Git blob ID | Purpose |
| --- | --- | --- |
| `assets/keymaps/default-comfy.json` | `6f5da0173b87ab9dc5aa4afd11a877de65488540` | Comfy graph keybindings |
| `assets/settings/default-comfy.json` | `f20fe6c7610d0c7ebe95def44c53d388c79b6607` | Comfy runtime defaults |

Neither file replaces or renames an upstream asset.

## Verification commands

The upstream path/blob audit enumerates `git ls-tree -r v1.16.1 -- assets` and
compares every entry with `git hash-object` on the working-tree path. The new-file
inventory compares the sorted working-tree paths with the v1.16.1 tree. Both
checks are also reflected by `git diff --name-status v1.16.1 -- assets`, whose
only results are the two new Comfy files above.

## Validation results

- `cargo test -p icons`: passed 2/2 exhaustive icon inventory tests.
- `cargo test -p ui components::image::tests`: passed 2/2 exhaustive vector inventory tests.
- `cargo check -p zed --features runtime-shaders,rust-tools`: passed.
- `cargo test -p zed --features runtime-shaders,rust-tools test_base_keymap`: passed 1/1.
- `ZED_STATELESS=1 cargo run -p zed --features runtime-shaders,rust-tools`: completed initialization, remained active for observation without asset-loading errors, and was intentionally stopped.
