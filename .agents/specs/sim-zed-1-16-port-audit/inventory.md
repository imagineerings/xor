# Sim Zed 1.16 port inventory

## Comparison refs

- Old base: `adc60ccf12e199b8828bad3abb2591e147034734` (`v1.10.2`)
- Old Sim tip: `d41ad2b582bceb6b1b49eb68f877ebed7d68eeb2` (`sim-dev` at audit start)
- New base: `eb8e1c8b5502b7007465fbbc465f4a736fa39210` (`v1.16.1`)
- Rebased tip: `5ab1c4de4a35e61476ff3cb88a5bcf7d9354d35e` (`sim-dev-reparented` at audit start)

## History limitation

The old base is **not** an ancestor of the old Sim tip, and the refs have no
merge base. A literal `git range-diff v1.10.2..sim-dev` therefore describes
the Sim repository's independent root history rather than a trustworthy port
series. This inventory compares the two endpoint trees instead.

The new base ancestry check is `valid`.

## Dispositions

- `preserved_exactly`: the old and rebased final tree entries are identical.
- `ported_with_adaptation`: both deltas touch the path, but final entries differ.
- `deleted_intentionally`: both final trees omit a path removed by the old delta.
- `missing_unresolved`: the old delta changes the path, the new delta does not, and final entries differ.
- `rebased_only`: only the new delta changes the path.

| Disposition | Paths |
| --- | ---: |
| `preserved_exactly` | 3622 |
| `ported_with_adaptation` | 286 |
| `deleted_intentionally` | 49 |
| `missing_unresolved` | 100 |
| `rebased_only` | 2 |

## Top-level mapping

| Area | Exact | Adapted | Deleted | Missing | Rebased only |
| --- | ---: | ---: | ---: | ---: | ---: |
| `.agents` | 429 | 0 | 1 | 0 | 0 |
| `.cargo` | 2 | 0 | 0 | 0 | 0 |
| `.factory` | 1 | 0 | 0 | 0 | 0 |
| `.git-blame-ignore-revs` | 1 | 0 | 0 | 0 | 0 |
| `.github` | 15 | 34 | 39 | 0 | 0 |
| `.gitignore` | 0 | 1 | 0 | 0 | 0 |
| `.rules` | 0 | 1 | 0 | 0 | 0 |
| `CONTRIBUTING.md` | 0 | 1 | 0 | 0 | 0 |
| `Cargo.lock` | 0 | 1 | 0 | 0 | 0 |
| `Cargo.lock.dev` | 1 | 0 | 0 | 0 | 0 |
| `Cargo.toml` | 0 | 1 | 0 | 0 | 0 |
| `LICENSE-APACHE` | 1 | 0 | 0 | 0 | 0 |
| `README.md` | 1 | 0 | 0 | 0 | 0 |
| `REVIEWERS.conl` | 1 | 0 | 0 | 0 | 0 |
| `assets` | 11 | 7 | 1 | 2 | 0 |
| `ci` | 1 | 0 | 0 | 0 | 0 |
| `crates` | 3026 | 200 | 6 | 90 | 1 |
| `docs` | 38 | 20 | 1 | 7 | 0 |
| `extensions` | 9 | 2 | 0 | 0 | 0 |
| `legal` | 2 | 0 | 0 | 0 | 0 |
| `nix` | 28 | 0 | 0 | 0 | 0 |
| `script` | 46 | 6 | 1 | 0 | 0 |
| `tooling` | 9 | 12 | 0 | 1 | 1 |

## Missing or unresolved paths

- `assets/keymaps/specific-overrides-macos.json`
- `assets/keymaps/specific-overrides.json`
- `crates/acp_thread/src/diff.rs`
- `crates/agent_ui/src/agent_registry_ui.rs`
- `crates/agent_ui/src/completion_provider.rs`
- `crates/agent_ui/src/entry_view_state.rs`
- `crates/bedrock/src/models.rs`
- `crates/buffer_diff/src/buffer_diff.rs`
- `crates/client/src/zed_urls.rs`
- `crates/cloud_api_types/src/cloud_api_types.rs`
- `crates/context_server/src/client.rs`
- `crates/context_server/src/transport/http.rs`
- `crates/copilot_ui/Cargo.toml`
- `crates/dev_container/src/devcontainer_manifest.rs`
- `crates/editor/src/bracket_colorization.rs`
- `crates/editor/src/clipboard.rs`
- `crates/editor/src/completions.rs`
- `crates/editor/src/config.rs`
- `crates/editor/src/element.rs`
- `crates/editor/src/git.rs`
- `crates/editor/src/git/blame.rs`
- `crates/editor/src/input.rs`
- `crates/fs/src/fake_git_repo.rs`
- `crates/fs/src/fs_watcher.rs`
- `crates/git/src/repository.rs`
- `crates/git_ui/src/commit_view.rs`
- `crates/git_ui/src/diff_multibuffer.rs`
- `crates/git_ui/src/file_diff_view.rs`
- `crates/git_ui/src/multi_diff_view.rs`
- `crates/git_ui/src/project_diff.rs`
- `crates/git_ui/src/solo_diff_view.rs`
- `crates/git_ui/src/staged_diff.rs`
- `crates/git_ui/src/text_diff_view.rs`
- `crates/git_ui/src/unstaged_diff.rs`
- `crates/git_ui/src/worktree_service.rs`
- `crates/gpui/src/elements/animation.rs`
- `crates/gpui/src/platform.rs`
- `crates/gpui_linux/src/linux/wayland/client.rs`
- `crates/gpui_linux/src/linux/wayland/window.rs`
- `crates/gpui_linux/src/linux/x11/window.rs`
- `crates/gpui_windows/src/dispatcher.rs`
- `crates/gpui_windows/src/events.rs`
- `crates/gpui_windows/src/platform.rs`
- `crates/language/src/buffer.rs`
- `crates/language/src/buffer_tests.rs`
- `crates/language/src/language_settings.rs`
- `crates/language_extension/src/extension_lsp_adapter.rs`
- `crates/language_extension/src/language_extension.rs`
- `crates/language_model/src/registry.rs`
- `crates/language_models/Cargo.toml`
- `crates/language_models/src/provider/anthropic.rs`
- `crates/language_models/src/provider/google.rs`
- `crates/language_models/src/provider/openai_subscribed.rs`
- `crates/language_tools/src/lsp_button.rs`
- `crates/languages/src/go.rs`
- `crates/markdown/Cargo.toml`
- `crates/markdown/src/html/html_rendering.rs`
- `crates/markdown/src/markdown.rs`
- `crates/mermaid_render/Cargo.toml`
- `crates/multi_buffer/src/multi_buffer.rs`
- `crates/node_runtime/src/node_runtime.rs`
- `crates/open_ai/src/responses.rs`
- `crates/opencode/src/opencode.rs`
- `crates/outline_panel/src/outline_panel.rs`
- `crates/picker/src/footer.rs`
- `crates/project/src/git_store/diff_buffer_list.rs`
- `crates/project/src/project_search.rs`
- `crates/project/src/search.rs`
- `crates/project/src/worktree_store.rs`
- `crates/project/tests/integration/context_server_store.rs`
- `crates/project/tests/integration/project_tests.rs`
- `crates/project_panel/Cargo.toml`
- `crates/project_panel/src/project_panel.rs`
- `crates/project_panel/src/project_panel_tests.rs`
- `crates/project_panel/src/tests/undo.rs`
- `crates/proto/proto/git.proto`
- `crates/proto/proto/worktree.proto`
- `crates/remote/src/transport/wsl.rs`
- `crates/remote_server/src/remote_editing_tests.rs`
- `crates/reqwest_client/src/reqwest_client.rs`
- `crates/search/src/buffer_search.rs`
- `crates/search/src/text_finder.rs`
- `crates/settings_content/src/terminal.rs`
- `crates/settings_ui/src/page_data.rs`
- `crates/settings_ui/src/pages/llm_providers_page.rs`
- `crates/terminal/src/terminal_settings.rs`
- `crates/ui/src/components/button/split_button.rs`
- `crates/ui/src/components/data_table.rs`
- `crates/vim/src/normal.rs`
- `crates/workspace/src/multi_workspace_tests.rs`
- `crates/worktree/src/worktree.rs`
- `crates/worktree/tests/integration/worktree_tests.rs`
- `docs/src/ai/edit-prediction.md`
- `docs/src/ai/use-a-gateway.md`
- `docs/src/ai/use-api-access.md`
- `docs/src/configuring-languages.md`
- `docs/src/languages/typescript.md`
- `docs/src/project-panel.md`
- `docs/theme/plugins.js`
- `tooling/lints/LICENSE-APACHE`

The complete path mapping and all four Git object IDs are in `port-ledger.csv`.
Adapted paths require build, test, specification, or manual evidence before they
can be called behaviorally equivalent.
