# Upstream inventory: Zed v1.10.2 to v1.11.3

## Range topology and accounting

- Verified `v1.10.2`: `adc60ccf12e199b8828bad3abb2591e147034734`.
- Verified `v1.11.3`: `952d712dac48a4af2c54fb22c82d82a9d69b72d4`.
- Merge base: `3648fe6f19644fa4fcbd4f1db3e5888efb8269c1`.
- The tags are sibling release lines, not an ancestor chain: the symmetric difference is 10 commits exclusive to v1.10.2 and 160 commits exclusive to v1.11.3.
- The requested right-hand range contains 160 non-merge commits and touches 437 path names when both sides of a rename are counted.
- The endpoint tree comparison contains 425 changed paths: 50 added, 372 modified, 2 deleted, and 1 rename; 28,677 insertions and 9,160 deletions.
- Twelve right-range path names are net-equal at the endpoints because equivalent changes were independently cherry-picked to the v1.10 stable line, reverted, or only changed release-channel state. They remain in the commit ledger.

The topology matters: commit reconciliation uses the 160 right-exclusive commits, while implementation is judged against endpoint behavior and explicitly preserves fixes already present in the v1.10.2 snapshot.

## Commit ledger

Each row is an independently reviewable upstream change with a durable ID. File status is commit-local. Hunk context is extracted from the actual patch; test evidence names changed upstream tests or states that no dedicated test file changed.

| ID | Upstream commit and behavior | Commit-local files | Symbols / hunk evidence | Upstream test evidence |
| --- | --- | --- | --- | --- |
| ZUP-001 | `b083358680a69ec982bd3efdcbcd1c90e08d6b2b` — Suppress `pet` log spam when opening Python projects (#60204) | M `crates/zlog/src/filter.rs` | `const DEFAULT_FILTERS: &[(&str, log::LevelFilter)] = &[` | No dedicated upstream test/fixture file changed. |
| ZUP-002 | `35c3d272828328b7217efccc146dc5b7d53490ff` — Bump Zed to v1.11.0 (#60209) | M `Cargo.lock`<br>M `crates/zed/Cargo.toml` | `name = "zed"` | No dedicated upstream test/fixture file changed. |
| ZUP-003 | `7e0f63412c60008f9dae7fcf65fc6ab6d7e0f957` — terminal_view: Use backslash escaping for dropped file paths for mac (#57747) | M `Cargo.lock`<br>M `crates/agent_ui/Cargo.toml`<br>M `crates/agent_ui/src/agent_panel.rs`<br>M `crates/terminal_view/Cargo.toml`<br>M `crates/terminal_view/src/terminal_view.rs` | `dependencies = [`<br>`semver.workspace = true`<br>`mod tests {`<br>`regex.workspace = true`<br>`schemars.workspace = true`<br>`shellexpand.workspace = true`<br>`impl TerminalView {` | No dedicated upstream test/fixture file changed. |
| ZUP-004 | `550ddc9405943cfd69f34646f7af0179a5b0be41` — agent_ui: Replace thread controls with slash commands (#59974) | M `assets/icons/folder_share.svg`<br>M `assets/icons/folder_shared.svg`<br>A `assets/icons/user_arrow_up.svg`<br>M `crates/agent_ui/src/agent_panel.rs`<br>M `crates/agent_ui/src/completion_provider.rs`<br>M `crates/agent_ui/src/conversation_view/thread_view.rs`<br>M `crates/agent_ui/src/message_editor.rs`<br>M `crates/icons/src/icons.rs` | `-<path d="M11.2045 14.4761V9.7034" stroke="#DCE0E5" stroke-width="1.65153" stroke-linecap="round" stroke-linejoin="round"/>`<br>`+<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">`<br>`impl AgentPanel {`<br>`impl PromptContextAction {`<br>`enum SlashCompletionCandidate {`<br>`impl SlashCompletionCandidate {`<br>`fn slash_completion_group_key(candidate: &SlashCompletionCandidate) -> u32 {`<br>`pub trait PromptCompletionProviderDelegate: Send + Sync + 'static {` | No dedicated upstream test/fixture file changed. |
| ZUP-005 | `bcd54d0adf13bf261c2d148c80941c51bd82ffa7` — editor: Add intelligent bracket colorization (#51580) | M `crates/editor/src/bracket_colorization.rs`<br>M `crates/editor/src/editor.rs`<br>A `crates/theme/src/color_space.rs`<br>M `crates/theme/src/theme.rs` | `+use std::cmp::Ordering;`<br>`use std::ops::Range;`<br>`use collections::{HashMap, HashSet};`<br>`use text::OffsetRangeExt as _;`<br>`impl Editor {`<br>`mod tests {`<br>`where`<br>`fn process_data«1()1» «1{` | No dedicated upstream test/fixture file changed. |
| ZUP-006 | `b206841b4b17f742d8a9028a526aea79ffb44360` — Add range-based whitespace and newline removal to buffer formatting (#53942) | M `crates/language/src/buffer.rs`<br>M `crates/language/src/buffer_tests.rs`<br>M `crates/project/src/lsp_store.rs` | `impl Buffer {`<br>`impl CharClassifier {`<br>`pub fn trailing_whitespace_ranges(rope: &Rope) -> Vec<Range<usize>> {`<br>`async fn test_normalize_whitespace(cx: &mut gpui::TestAppContext) {`<br>`fn test_trailing_whitespace_ranges(mut rng: StdRng) {`<br>`impl LocalLspStore {` | `crates/language/src/buffer_tests.rs` |
| ZUP-007 | `d0802abdecadabc5c3248ebf75a466831f6dfbe4` — grammars: Fix runnable gutter detection for `describe.skipIf` / `test.skipIf` in JS/TS/TSX (#60153) | M `crates/grammars/src/javascript/outline.scm`<br>M `crates/grammars/src/javascript/runnables.scm`<br>M `crates/grammars/src/tsx/outline.scm`<br>M `crates/grammars/src/tsx/runnables.scm`<br>M `crates/grammars/src/typescript/outline.scm`<br>M `crates/grammars/src/typescript/runnables.scm`<br>M `crates/languages/src/typescript.rs` | `+; Also matches direct modifiers: .skip, .todo, .only, .failing (Jest, Bun, Vitest)`<br>`-; Add support for parameterized tests`<br>`-    (#eq? @_property "each"))`<br>`-    (#any-of? @_property "each"))`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-008 | `10b07951838e422722e34641f4a9c0bfec9037ff` — sandbox: Fix WSL downloaded binary path (#60210) | M `crates/sandbox/src/windows_wsl.rs` | `const HELPER_RESULT_PREFIX: &str = "zed-wsl-helper:";`<br>`want="$channel $version"`<br>`tar -xzf "$tarball" -C "$tmp/unpacked"`<br>`if [ -z "$helper_src" ]; then`<br>`printf '%s' "$want" > "$marker"` | No dedicated upstream test/fixture file changed. |
| ZUP-009 | `995e56d2639eff49ebdb6e988d01f1593b14aff5` — markdown_preview: Use UI font for base font size (#60212) | M `assets/settings/default.json`<br>M `crates/markdown/src/markdown.rs`<br>M `crates/settings_content/src/theme.rs`<br>M `crates/theme_settings/src/settings.rs` | `-  // The default font size for the markdown preview. Falls back to the editor font size if unset.`<br>`mod tests {`<br>`pub struct ThemeSettingsContent {`<br>`pub struct ThemeSettings {`<br>`impl ThemeSettings {` | No dedicated upstream test/fixture file changed. |
| ZUP-010 | `d132afe9fc440dc6822d6cba7ee34fa1d71056de` — Add new area labels to track mapping - 2 (#60247) | M `script/community-pr-track-mapping.json` | `-      "labels": ["area:controls/ime", "area:controls/mouse", "area:gpui"]`<br>`+        "area:text finder",`<br>`+        "area:ai/agent thread/sandbox",` | No dedicated upstream test/fixture file changed. |
| ZUP-011 | `7d545c0baeff67d3427e53cc4f382eeb967e4119` — markdown: Make linked images clickable (#59525) | M `Cargo.lock`<br>M `crates/markdown/Cargo.toml`<br>M `crates/markdown/src/markdown.rs` | `dependencies = [`<br>`gpui_platform = { workspace = true, features = ["wayland", "x11"] }`<br>`pub struct MarkdownElement {`<br>`impl MarkdownElement {`<br>`fn collect_image_alt_text(`<br>`fn image_fallback_element(dest_url: SharedString, alt_text: Option<SharedString>`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-012 | `d1e8c0b50f445ae34c4e49d59be88dab052f0d1a` — git_store: Avoid redundant work in worktree git repository update (#60205) | M `crates/project/src/git_store.rs` | `impl GitStore {` | No dedicated upstream test/fixture file changed. |
| ZUP-013 | `f964172a69779353853b7aa63c944c9fa680ab7e` — project_panel: Wrap filenames in code spans in confirmation dialogs (#53068) | M `crates/project_panel/src/project_panel.rs`<br>M `crates/project_panel/src/project_panel_tests.rs`<br>M `crates/workspace/src/pane.rs` | `impl ProjectPanel {`<br>`async fn test_delete_prompt_escapes_markdown_in_file_name(cx: &mut gpui::TestApp`<br>`use util::{`<br>`fn dirty_message_for(buffer_path: Option<ProjectPath>, path_style: PathStyle) ->`<br>`mod tests {` | `crates/project_panel/src/project_panel_tests.rs` |
| ZUP-014 | `779ca5ef7248b1f66bad91f85a5c5d0b6922de6e` — docs: Add note to Windows building docs (#60104) | M `docs/src/development/windows.md` | `Clone the [Zed repository](https://github.com/zed-industries/zed).` | No dedicated upstream test/fixture file changed. |
| ZUP-015 | `9cb3140ea10dbbf3ca5ece1ab9daf9ec3f44ba0b` — wgpu: Use wgpu from crates.io (#60264) | M `Cargo.lock`<br>M `Cargo.toml` | `name = "naga"`<br>`name = "wgpu"`<br>`name = "wgpu-core"`<br>`name = "wgpu-core-deps-apple"`<br>`name = "wgpu-core-deps-emscripten"`<br>`name = "wgpu-core-deps-windows-linux-android"`<br>`name = "wgpu-hal"`<br>`name = "wgpu-naga-bridge"` | No dedicated upstream test/fixture file changed. |
| ZUP-016 | `b3d5ead59f39bdd7d5badf1a0a968522f960fa12` — Add Guild board automation (#60266) | A `.github/workflows/guild_assignment_status.yml`<br>A `.github/workflows/guild_new_pr_notify.yml`<br>A `.github/workflows/guild_stale_assignments.yml`<br>A `.github/workflows/guild_weekly_shipped.yml`<br>M `.github/workflows/pr_issue_labeler.yml`<br>M `.github/workflows/slack_notify_community_automation_failure.yml`<br>M `script/github-community-pr-board.py`<br>A `script/github-guild-board.py` | `+# Guild board (https://github.com/orgs/zed-industries/projects/74) reactions to issue events:`<br>`+# When a guild member opens a PR while already having another open PR in`<br>`+# Scheduled sweep of the Guild board (https://github.com/orgs/zed-industries/projects/74).`<br>`+# Scheduled Slack digest of Guild board`<br>`jobs:`<br>`on:`<br>`def compute_contributor(pr_labels):`<br>`+#!/usr/bin/env python3` | No dedicated upstream test/fixture file changed. |
| ZUP-017 | `9375695626f1bba18ae5ba3153aa200c999e342e` — Project/use line hint for search (#58871) | M `crates/project/src/project.rs`<br>M `crates/project/src/project_search.rs`<br>M `crates/project/src/search.rs`<br>M `crates/remote_server/src/headless_project.rs` | `impl Project {`<br>`use gpui::{App, AppContext, AsyncApp, BackgroundExecutor, Entity, Priority, Task`<br>`use crate::{`<br>`pub struct SearchResultsHandle {`<br>`impl SearchResultsHandle {`<br>`impl Search {`<br>`struct Worker {`<br>`impl RequestHandler<'_> {` | No dedicated upstream test/fixture file changed. |
| ZUP-018 | `da2b62c917f5171eb0bf1bb1ec3168e5d6093b99` — Fix relative line number calculation when the first row is wrapped (#53759) | M `crates/editor/src/editor.rs`<br>M `crates/editor/src/element.rs` | `impl EditorSnapshot {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-019 | `e5513539ec4f2b9709c55060aa84fc4a536ee031` — editor: Accumulate consecutive KillRingCut line kills (#51761) | M `crates/editor/src/clipboard.rs`<br>M `crates/editor/src/editor_tests.rs` | `impl Editor {`<br>`impl Global for KillRing {}`<br>`async fn test_cut_line_ends(cx: &mut TestAppContext) {` | `crates/editor/src/editor_tests.rs` |
| ZUP-020 | `4a1cb2b1e24cf25463e29f7aa67980d4d86ebabc` — lsp_button: Fix missing server metadata after restart (#55162) | M `crates/language_tools/src/lsp_button.rs` | `impl LanguageServers {`<br>`impl LspButton {`<br>`impl Render for LspButton {` | No dedicated upstream test/fixture file changed. |
| ZUP-021 | `21d66eb9d593dafdf9d878ddeae0e7fcd08549df` — settings: Remove unused `message_editor` setting (#60260) | M `assets/settings/default.json`<br>M `crates/settings/src/vscode_import.rs`<br>M `crates/settings_content/src/settings_content.rs`<br>M `crates/settings_ui/src/page_data.rs` | `-  "message_editor": {`<br>`impl VsCodeSettings {`<br>`pub struct SettingsContent {`<br>`pub struct PanelSettingsContent {`<br>`fn language_settings_data() -> Box<[SettingsPageItem]> {` | No dedicated upstream test/fixture file changed. |
| ZUP-022 | `c55693876ee2e8b3868a2287b22be1c1394058a3` — cloud_api_types: Remove unused `AcceptTermsOfServiceResponse` type (#60226) | M `crates/cloud_api_types/src/cloud_api_types.rs` | `pub struct OrganizationEditPredictionConfiguration {` | No dedicated upstream test/fixture file changed. |
| ZUP-023 | `52b61cf424b3d731994dcde1477374b5d8e49160` — zed: Respect `default_open_behavior` when opening file from Finder (#59661) | M `crates/zed/src/zed/open_listener.rs` | `pub(crate) fn open_options_for_request(`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-024 | `442a3476bcaf8090e8e7ecaa4c0c1254236ff10f` — Fix crash when trashing all untracked files (#60235) | M `crates/fs/src/fs.rs` | `impl Fs for RealFs {` | No dedicated upstream test/fixture file changed. |
| ZUP-025 | `2ec0e6c2d7024f684e8bc23ce5882419b81f7f11` — editor: Fix panic when confirming completion across buffers (#59471) | M `crates/editor/src/completions.rs`<br>M `crates/editor/src/editor_tests.rs` | `impl Editor {`<br>`async fn test_completion_in_multibuffer_with_replace_range(cx: &mut TestAppConte` | `crates/editor/src/editor_tests.rs` |
| ZUP-026 | `969c6c719ca10102903590e15601c55b57a8838e` — editor: Fix panic when restoring empty selections on undo/redo (#59372) | M `crates/editor/src/editor.rs`<br>M `crates/editor/src/editor_tests.rs`<br>M `crates/vim/src/command.rs`<br>M `crates/vim/src/normal.rs` | `struct DeferredSelectionEffectsState {`<br>`struct SelectionHistory {`<br>`impl SelectionHistory {`<br>`impl Editor {`<br>`fn test_undo_redo_with_selection_restoration(cx: &mut TestAppContext) {`<br>`pub fn register(editor: &mut Editor, cx: &mut Context<Vim>) {`<br>`pub(crate) fn register(editor: &mut Editor, cx: &mut Context<Vim>) {` | `crates/editor/src/editor_tests.rs` |
| ZUP-027 | `7eb4cb2bfa02618761389d5e760a9a9b6763f301` — workspace: Fix panic when discarding a draft workspace (#60279) | M `crates/workspace/src/multi_workspace.rs`<br>M `crates/workspace/src/multi_workspace_tests.rs` | `impl MultiWorkspace {`<br>`async fn test_find_or_create_workspace_uses_project_group_key_when_paths_are_mis` | `crates/workspace/src/multi_workspace_tests.rs` |
| ZUP-028 | `78b6bf2fbe2aa46688507e839ac1b537522e4c58` — command_palette: Show scrollbar in command palette (#60239) | M `crates/command_palette/src/command_palette.rs` | `impl CommandPalette {` | No dedicated upstream test/fixture file changed. |
| ZUP-029 | `17090674b34288db75128f96dfb336116e058ff2` — text_finder: Add collapsible file groups (#60193) | M `assets/keymaps/specific-overrides-macos.json`<br>M `assets/keymaps/specific-overrides.json`<br>M `crates/search/src/text_finder.rs`<br>M `crates/search/src/text_finder/delegate.rs`<br>M `crates/search/src/text_finder/render.rs` | `+      "alt-cmd-[": "text_finder::Fold",`<br>`+      "ctrl-{": "text_finder::Fold",`<br>`use crate::{ProjectSearchView, SearchOptions, text_finder::delegate::PopulatePro`<br>`impl TextFinder {`<br>`use gpui::{`<br>`use ui::{`<br>`use workspace::item::ItemSettings;`<br>`pub struct Delegate {` | No dedicated upstream test/fixture file changed. |
| ZUP-030 | `4ff55d09e5ccd12fd67c5b5c17b0ffea703a4db0` — Refactor agent settings UI code (#60274) | M `crates/acp_thread/src/connection.rs`<br>M `crates/agent_ui/src/conversation_view.rs`<br>M `crates/agent_ui/src/conversation_view/thread_view.rs`<br>M `crates/language_model/src/fake_provider.rs`<br>M `crates/language_model/src/language_model.rs`<br>M `crates/language_models/src/provider/anthropic.rs`<br>M `crates/language_models/src/provider/anthropic_compatible.rs`<br>M `crates/language_models/src/provider/bedrock.rs`<br>M `crates/language_models/src/provider/cloud.rs`<br>M `crates/language_models/src/provider/copilot_chat.rs`<br>M `crates/language_models/src/provider/deepseek.rs`<br>M `crates/language_models/src/provider/google.rs`<br>M `crates/language_models/src/provider/llama_cpp.rs`<br>M `crates/language_models/src/provider/lmstudio.rs`<br>M `crates/language_models/src/provider/mistral.rs`<br>M `crates/language_models/src/provider/ollama.rs`<br>M `crates/language_models/src/provider/open_ai.rs`<br>M `crates/language_models/src/provider/open_ai_compatible.rs`<br>M `crates/language_models/src/provider/open_router.rs`<br>M `crates/language_models/src/provider/openai_subscribed.rs`<br>M `crates/language_models/src/provider/opencode.rs`<br>M `crates/language_models/src/provider/vercel_ai_gateway.rs`<br>M `crates/language_models/src/provider/x_ai.rs`<br>M `crates/settings_ui/src/pages/llm_providers_page.rs`<br>M `crates/settings_ui/src/settings_ui.rs`<br>M `crates/zed_actions/src/lib.rs` | `use gpui::{Entity, SharedString, Task};`<br>`pub struct AuthRequired {`<br>`impl AuthRequired {`<br>`use gpui::{`<br>`use language::{Buffer, Language, Rope};`<br>`enum AuthState {`<br>`impl ConversationView {`<br>`impl Render for ConversationView {` | No dedicated upstream test/fixture file changed. |
| ZUP-031 | `e1ec575d3f78968c8cbc4168cdda40cac5e2aa8c` — editor: Update gutter hover tooltip on modifier changes (#58880) | M `crates/editor/src/editor.rs`<br>M `crates/editor/src/editor_tests.rs` | `use ui::{`<br>`pub struct RewrapOptions {`<br>`impl Editor {`<br>`fn add_log_breakpoint_at_cursor(` | `crates/editor/src/editor_tests.rs` |
| ZUP-032 | `27a9e2057b71ef84a1dbe06fc51ac04c62f420d4` — vim: Fix Helix cursor position when deleting to end of line (#59987) | M `crates/vim/src/helix.rs`<br>M `crates/vim/src/visual.rs` | `mod test {`<br>`impl Vim {` | No dedicated upstream test/fixture file changed. |
| ZUP-033 | `31fc9d5f4710e30a4908525f6f0b930fce71e6f6` — project_panel: Continue batch delete when individual entries fail (#59595) | M `Cargo.lock`<br>M `crates/fs/src/fs.rs`<br>M `crates/project_panel/Cargo.toml`<br>M `crates/project_panel/src/project_panel.rs`<br>M `crates/project_panel/src/tests/undo.rs` | `dependencies = [`<br>`impl FakeFs {`<br>`fs.workspace = true`<br>`fn get_item_color(is_sticky: bool, cx: &App) -> ItemColors {`<br>`impl ProjectPanel {`<br>`async fn trash_undo_redo(cx: &mut gpui::TestAppContext) {` | `crates/project_panel/src/tests/undo.rs` |
| ZUP-034 | `ea87b0579464067eb45a1c1a1f2c1bdb80af7e1f` — Fix worktree grouping for bare checkouts (#59968) | M `crates/collab/src/db.rs`<br>M `crates/collab/src/rpc.rs`<br>M `crates/project/src/project.rs`<br>M `crates/project/src/worktree_store.rs`<br>M `crates/project/tests/integration/project_tests.rs`<br>M `crates/proto/proto/call.proto`<br>M `crates/proto/proto/worktree.proto`<br>M `crates/proto/src/proto.rs`<br>M `crates/remote_server/src/headless_project.rs`<br>M `crates/remote_server/src/remote_editing_tests.rs`<br>M `crates/workspace/src/multi_workspace_tests.rs`<br>M `crates/worktree/src/worktree.rs`<br>M `crates/worktree/tests/integration/worktree_tests.rs` | `impl RejoinedProject {`<br>`fn notify_rejoined_projects(`<br>`async fn join_project(`<br>`impl Project {`<br>`impl WorktreeStore {`<br>`async fn test_project_group_keys_remain_distinct_for_sibling_repo_subdirectories`<br>`message UpdateWorktree {`<br>`message AddWorktreeResponse {` | `crates/project/tests/integration/project_tests.rs`<br>`crates/remote_server/src/remote_editing_tests.rs`<br>`crates/workspace/src/multi_workspace_tests.rs`<br>`crates/worktree/tests/integration/worktree_tests.rs` |
| ZUP-035 | `4aa8ad9742b1ee948d64429a5814d9b9a861350a` — agent: Make terminal threads searchable (#60292) | M `assets/keymaps/default-linux.json`<br>M `assets/keymaps/default-macos.json`<br>M `assets/keymaps/default-windows.json`<br>M `crates/agent_ui/src/agent_panel.rs` | `+  {`<br>`use settings::{NotifyWhenAgentWaiting, Settings, update_settings_file};`<br>`use workspace::{`<br>`struct AgentTerminal {`<br>`impl AgentPanel {`<br>`impl Render for AgentPanel {` | No dedicated upstream test/fixture file changed. |
| ZUP-036 | `2882636c06923e58d83865ecc370bd0d8199d738` — Fix hanging updates after system sleep (#60301) | M `crates/auto_update/src/auto_update.rs`<br>M `crates/gpui/src/app.rs`<br>M `crates/reqwest_client/src/reqwest_client.rs` | `pub struct AutoUpdater {`<br>`impl AutoUpdater {`<br>`impl Application {`<br>`pub struct App {`<br>`impl App {`<br>`impl ReqwestClient {` | No dedicated upstream test/fixture file changed. |
| ZUP-037 | `bb48a42983f2a4bb9ac9d31c63abe02497088f67` — keymap_editor: Fix deleting and editing bindings that use deprecated action names (#60300) | M `crates/keymap_editor/src/keymap_editor.rs`<br>M `crates/settings/src/keymap_file.rs` | `impl KeymapEditor {`<br>`impl KeybindingEditorModal {`<br>`async fn save_keybinding_update(`<br>`async fn remove_keybinding(`<br>`mod tests {`<br>`impl KeymapFile {` | No dedicated upstream test/fixture file changed. |
| ZUP-038 | `b5c2d8a13f395bbdbaf9cb74bda16bbcb00414d1` — Revert "Fix hanging updates after system sleep (#60301)" (#60321) | M `crates/auto_update/src/auto_update.rs`<br>M `crates/gpui/src/app.rs`<br>M `crates/reqwest_client/src/reqwest_client.rs` | `pub struct AutoUpdater {`<br>`impl AutoUpdater {`<br>`impl Application {`<br>`pub struct App {`<br>`impl App {`<br>`impl ReqwestClient {` | No dedicated upstream test/fixture file changed. |
| ZUP-039 | `552fc9f3c3c775276c2ce3f0fb93f1f4c2c18ba6` — reqwest_client: Fix streamed request bodies being truncated (#60314) | M `crates/reqwest_client/src/reqwest_client.rs` | `impl futures::Stream for StreamReader {`<br>`pub fn poll_read_buf(`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-040 | `0346a17d097eabeb0f34f21f0a75fdd957bbf298` — Add zed: get merch command palette action (#60330) | M `crates/zed/src/zed.rs`<br>M `crates/zed_actions/src/lib.rs` | `use zed_actions::{`<br>`const STATUS_URL: &str = "https://status.zed.dev";`<br>`fn register_actions(`<br>`actions!(` | No dedicated upstream test/fixture file changed. |
| ZUP-041 | `f961889ad4d23ca0cf157a3f963bdb1159f27cce` — Fix excluded language servers starting nonetheless (#60000) | M `crates/language/src/language_settings.rs`<br>M `crates/settings_content/src/language.rs` | `pub use settings::{`<br>`impl LanguageSettings {`<br>`mod tests {`<br>`impl merge_from::MergeFrom for AllLanguageSettingsContent {`<br>`pub enum SoftWrap {`<br>`mod test {` | No dedicated upstream test/fixture file changed. |
| ZUP-042 | `4a3b763518c75c3d991985bd4eee822a56efc827` — Add progress bar to "Downloading Zed Update..." button (#60294) | M `crates/auto_update/src/auto_update.rs`<br>M `crates/title_bar/src/update_version.rs`<br>M `crates/ui/src/components/collab/update_button.rs`<br>M `crates/ui/src/components/progress/circular_progress.rs` | `use smol::fs::File;`<br>`pub enum AutoUpdateStatus {`<br>`impl PartialEq for AutoUpdateStatus {`<br>`impl AutoUpdater {`<br>`async fn download_release(`<br>`mod tests {`<br>`impl UpdateVersion {`<br>`impl Render for UpdateVersion {` | No dedicated upstream test/fixture file changed. |
| ZUP-043 | `a91c8aa7d71215bff8c4bed29085a7cff7bd80ad` — Remove more storybook leftovers (#60337) | M `.github/CODEOWNERS.hold`<br>D `assets/keymaps/storybook.json`<br>M `script/check-keymaps`<br>D `script/storybook` | `-/crates/storybook/ @zed-industries/ui-team`<br>`-[`<br>`result=$(git grep --no-color --line-number --fixed-strings -e "$pattern" -- \`<br>`-#!/usr/bin/env bash` | No dedicated upstream test/fixture file changed. |
| ZUP-044 | `98ddc3ae2efef817d71f40f8ead59b87e5407d68` — Update openssl dependencies (#60342) | M `Cargo.lock` | `name = "openssl"`<br>`source = "registry+https://github.com/rust-lang/crates.io-index"`<br>`name = "openssl-sys"` | No dedicated upstream test/fixture file changed. |
| ZUP-045 | `59185f5a70b4ed7015de301db494b6e1032e9a09` — livekit_api: Fix LiveKit token revocation timestamps (#60157) | M `crates/collab/tests/integration/channel_tests.rs`<br>M `crates/collab/tests/integration/test_server.rs`<br>M `crates/livekit_api/Cargo.toml`<br>M `crates/livekit_api/src/livekit_api.rs`<br>M `crates/livekit_api/src/token.rs`<br>M `crates/livekit_client/Cargo.toml`<br>M `crates/livekit_client/src/test.rs` | `async fn test_channel_room(`<br>`use workspace::{MultiWorkspace, Workspace, WorkspaceStore};`<br>`pub struct TestServer {`<br>`impl TestServer {`<br>`doctest = false`<br>`pub struct LiveKitClient {`<br>`impl LiveKitClient {`<br>`impl Client for LiveKitClient {` | `crates/collab/tests/integration/channel_tests.rs`<br>`crates/collab/tests/integration/test_server.rs`<br>`crates/livekit_client/src/test.rs` |
| ZUP-046 | `616b76cd5912441676e5f015c084a9d57b6c0cfb` — Update Danger to 13.0.8 (#60346) | M `script/danger/package.json`<br>M `script/danger/pnpm-lock.yaml` | `-    "danger": "13.0.7",`<br>`importers:`<br>`packages:`<br>`snapshots:` | No dedicated upstream test/fixture file changed. |
| ZUP-047 | `9448417157a9e690d87213c89ea9913803373b4f` — project_symbols: Add preview to project symbols picker (#59863) | M `Cargo.lock`<br>M `crates/picker/src/preview.rs`<br>M `crates/picker_preview/Cargo.toml`<br>M `crates/picker_preview/src/picker_preview.rs`<br>M `crates/project_symbols/Cargo.toml`<br>M `crates/project_symbols/src/project_symbols.rs` | `dependencies = [`<br>`use language::{Anchor, Buffer, HighlightedText};`<br>`pub enum PreviewSource {`<br>`impl Update {`<br>`doctest = false`<br>`use gpui::{`<br>`use picker::{`<br>`use ui::{ActiveTheme, Color, div, prelude::*, v_flex};` | No dedicated upstream test/fixture file changed. |
| ZUP-048 | `7ed553b391393ee4e62782e710605778e2200a18` — acp_thread: Shrink ACP terminal scrollback to used on exit (#60019) | M `crates/acp_thread/src/acp_thread.rs`<br>M `crates/terminal/src/alacritty.rs`<br>M `crates/terminal/src/terminal.rs` | `impl AcpThread {`<br>`mod tests {`<br>`pub(super) fn clear_saved_screen(term: &mut Term<ZedListener>) {`<br>`use crate::alacritty::{`<br>`impl Terminal {` | No dedicated upstream test/fixture file changed. |
| ZUP-049 | `be7e5b0338dbb49170913ba539c4e743d9d071f4` — terminal_view: Show terminal inline assist keybinding in tooltip (#55903) | M `crates/terminal_view/src/terminal_panel.rs` | `use gpui::{`<br>`pub struct TerminalPanel {`<br>`impl TerminalPanel {`<br>`impl workspace::TerminalProvider for TerminalProvider {`<br>`struct InlineAssistTabBarButton {`<br>`impl Render for InlineAssistTabBarButton {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-050 | `4da73e19700e2ea8bff391243e43777e807527ad` — git_ui: Add configure and docs links to commit message tooltip (#60357) | M `Cargo.lock`<br>M `crates/client/src/zed_urls.rs`<br>M `crates/git_ui/Cargo.toml`<br>M `crates/git_ui/src/git_panel.rs`<br>M `crates/ui/src/components/button/icon_button.rs` | `dependencies = [`<br>`pub fn skills_docs(cx: &App) -> String {`<br>`call = { workspace = true, optional = true }`<br>`fs.workspace = true`<br>`futures-lite.workspace = true`<br>`release_channel.workspace = true`<br>`remote.workspace = true`<br>`time_format.workspace = true` | No dedicated upstream test/fixture file changed. |
| ZUP-051 | `b497e2024aebe0adeefde670f6f3e1ef9c4257a7` — Add Claude Fable 5 to Amazon Bedrock (#59016) | M `crates/bedrock/src/models.rs`<br>M `crates/language_models/src/provider/bedrock.rs` | `pub enum Model {`<br>`impl Model {`<br>`mod tests {`<br>`impl LanguageModel for BedrockModel {` | No dedicated upstream test/fixture file changed. |
| ZUP-052 | `aabcd71690db4d5608eef8fcca4623b3bb10f7b1` — Improve commit container at larger font sizes (#60331) | M `crates/git_ui/src/git_panel.rs` | `impl GitPanel {`<br>`pub fn panel_editor_container(_window: &mut Window, cx: &mut App) -> Div {`<br>`impl RenderOnce for PanelRepoFooter {` | No dedicated upstream test/fixture file changed. |
| ZUP-053 | `f4364d870e8e805ff272ab25a198ab46db19e51b` — gpui_linux: Add support for open_window in headless client (#60359) | M `crates/gpui_linux/src/linux/headless.rs`<br>M `crates/gpui_linux/src/linux/headless/client.rs`<br>A `crates/gpui_linux/src/linux/headless/window.rs` | `mod client;`<br>`use gpui_util::ResultExt;`<br>`pub struct HeadlessClientState {`<br>`impl HeadlessClient {`<br>`impl LinuxClient for HeadlessClient {`<br>`+//! Windows for the headless platform client.` | No dedicated upstream test/fixture file changed. |
| ZUP-054 | `53552b29a44275428a3a990ee1d4a9b37e728331` — docs: Clarify agent notification placement (#54032) | M `docs/src/ai/agent-panel.md` | `If you send a prompt to the Agent and then put Zed in the background, you can ch` | No dedicated upstream test/fixture file changed. |
| ZUP-055 | `814b152a0f40b6bc64996e8f728c9c3de8e8104e` — bedrock: Add Claude Sonnet 5 (#60360) | M `crates/bedrock/src/models.rs` | `pub enum Model {`<br>`impl Model {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-056 | `6eaad52c29a026591b18acfc0ef6f35bce85d676` — language_models: Avoid sending Bedrock cache-point-only messages (#59436) | M `crates/language_models/src/provider/bedrock.rs` | `pub fn into_bedrock(`<br>`impl ConfigurationView {` | No dedicated upstream test/fixture file changed. |
| ZUP-057 | `262fb9ba2a58859c2eef7814249f4d266b065179` — Show per-file diff stat in multibuffer headers (#60299) | M `crates/editor/src/element/header.rs` | `use ui::{`<br>`pub(crate) fn render_buffer_header(` | No dedicated upstream test/fixture file changed. |
| ZUP-058 | `12e1e24434ebecaa5d81e6d8f08f4c9c4f8cbc14` — Initiate MCP OAuth flow on post-initialize 401 responses (#60236) | M `crates/context_server/src/client.rs`<br>M `crates/context_server/src/context_server.rs`<br>M `crates/context_server/src/protocol.rs`<br>M `crates/context_server/src/transport.rs`<br>M `crates/context_server/src/transport/http.rs`<br>M `crates/project/src/context_server_store.rs`<br>M `crates/project/tests/integration/context_server_store.rs` | `use parking_lot::Mutex;`<br>`use crate::{`<br>`pub(crate) struct Client {`<br>`impl Client {`<br>`use url::Url;`<br>`impl ContextServer {`<br>`use anyhow::Result;`<br>`use crate::client::{Client, NotificationSubscription};` | `crates/project/tests/integration/context_server_store.rs` |
| ZUP-059 | `2f1caadd387f403f675a131e2841d1f85e465bf3` — Fix expanded commit editor (#60368) | M `crates/git_ui/src/git_panel.rs` | `impl GitPanel {` | No dedicated upstream test/fixture file changed. |
| ZUP-060 | `9eb8a7c0add822e8bc7dac64f7929be0124a2fe4` — Handle cases where Dockerfile aliases are chained (#57552) | M `crates/dev_container/src/devcontainer_manifest.rs` | `chmod +x ./install.sh`<br>`fn dockerfile_inject_alias(`<br>`fn image_from_dockerfile(dockerfile_contents: String, target: &Option<String>) -`<br>`mod test {`<br>`FROM ${IMAGE} AS production`<br>`RUN echo $RUBY_VERSION2` | No dedicated upstream test/fixture file changed. |
| ZUP-061 | `fa8540ff6299c30ebb325601ad821127a314f44e` — Update Wasmtime dependencies (#60341) | M `Cargo.lock`<br>M `Cargo.toml` | `dependencies = [`<br>`name = "cranelift-assembler-x64"`<br>`source = "registry+https://github.com/rust-lang/crates.io-index"`<br>`name = "cranelift-assembler-x64-meta"`<br>`name = "cranelift-bforest"`<br>`name = "cranelift-bitset"`<br>`name = "cranelift-codegen"`<br>`name = "cranelift-codegen-meta"` | No dedicated upstream test/fixture file changed. |
| ZUP-062 | `5a823cf70ebb1d7a158c6a7ca455860cd9f6aed0` — docs: Add new text finder (#60189) | M `docs/src/finding-navigating.md` | `Open any file in your project with {#kb file_finder::Toggle}. Type part of the f`<br>`Quickly switch between open tabs with {#kb tab_switcher::Toggle}. Tabs are sorte` | No dedicated upstream test/fixture file changed. |
| ZUP-063 | `ea3d0f7abeb3dc5d0954d6d3fff453af5b0c7af9` — keymap: Avoid format-vs-rules collision in JetBrains overlay (#55364) | M `assets/keymaps/linux/jetbrains.json`<br>M `assets/keymaps/macos/jetbrains.json` | `+  {` | No dedicated upstream test/fixture file changed. |
| ZUP-064 | `5aa6e8a0b37a46828325e9f4f01ce3e0138017b3` — Reduce OAuth scope to avoid Windows credential size limit (#58541) | M `crates/gpui_windows/src/platform.rs`<br>M `crates/language_models/src/provider/openai_subscribed.rs` | `impl Platform for WindowsPlatform {`<br>`async fn do_oauth_flow(` | No dedicated upstream test/fixture file changed. |
| ZUP-065 | `c56646ffdfffc27d0aab8b3940ef93c4153375be` — terminal: Fix IME candidate window not following cursor in TUI apps (#59911) | M `crates/gpui_linux/src/linux/wayland/client.rs`<br>M `crates/terminal_view/src/terminal_element.rs`<br>M `crates/terminal_view/src/terminal_view.rs` | `impl WaylandClientStatePtr {`<br>`impl Element for TerminalElement {`<br>`struct TerminalInputHandler {`<br>`impl InputHandler for TerminalInputHandler {`<br>`fn subscribe_for_terminal_events(` | No dedicated upstream test/fixture file changed. |
| ZUP-066 | `b77ec90b2e6585622099235d9fb7d708d22ad956` — git: Do not recompute git_access on every file change (#59521) | M `crates/git_ui/src/git_panel.rs`<br>M `crates/project/src/git_store.rs` | `pub struct GitPanel {`<br>`impl GitPanel {`<br>`enum DiffKind {`<br>`pub enum GitAccess {`<br>`pub enum RepositoryEvent {`<br>`impl GitStore {` | No dedicated upstream test/fixture file changed. |
| ZUP-067 | `4b7369481dcf36f22ff8f813d411e1d296aebe57` — Add dylint lint library for Zed-specific patterns (#58496) | A `.agents/skills/lint-creator/SKILL.md`<br>M `Cargo.toml`<br>A `tooling/lints/.cargo/config.toml`<br>A `tooling/lints/.gitignore`<br>A `tooling/lints/Cargo.toml`<br>A `tooling/lints/README.md`<br>A `tooling/lints/rust-toolchain.toml`<br>A `tooling/lints/single-lint`<br>A `tooling/lints/src/blocking_io_on_foreground.rs`<br>A `tooling/lints/src/entity_update_in_render.rs`<br>A `tooling/lints/src/lib.rs`<br>A `tooling/lints/src/notify_in_render.rs`<br>A `tooling/lints/src/owned_string_into_shared.rs`<br>A `tooling/lints/src/render_helpers.rs`<br>A `tooling/lints/test_fixture/Cargo.toml`<br>A `tooling/lints/test_fixture/consumer/Cargo.toml`<br>A `tooling/lints/test_fixture/consumer/src/lib.rs`<br>A `tooling/lints/test_fixture/gpui/Cargo.toml`<br>A `tooling/lints/test_fixture/gpui/src/lib.rs`<br>A `tooling/lints/test_fixture/gpui_shared_string/Cargo.toml`<br>A `tooling/lints/test_fixture/gpui_shared_string/src/lib.rs`<br>A `tooling/lints/test_fixture/render_consumer/Cargo.toml`<br>A `tooling/lints/test_fixture/render_consumer/src/lib.rs`<br>A `tooling/lints/ui/async_block_without_await.rs`<br>A `tooling/lints/ui/async_block_without_await.stderr`<br>A `tooling/lints/ui/blocking_io_on_foreground.rs`<br>A `tooling/lints/ui/blocking_io_on_foreground.stderr`<br>A `tooling/lints/ui/entity_update_in_render.rs`<br>A `tooling/lints/ui/entity_update_in_render.stderr`<br>A `tooling/lints/ui/owned_string_into_shared.rs`<br>A `tooling/lints/ui/owned_string_into_shared.stderr` | `+---`<br>`ignored = [`<br>`+[target.'cfg(all())']`<br>`+/target/`<br>`+[package]`<br>`+# lints`<br>`+[toolchain]`<br>`+#!/usr/bin/env bash` | `tooling/lints/ui/async_block_without_await.stderr`<br>`tooling/lints/ui/blocking_io_on_foreground.stderr`<br>`tooling/lints/ui/entity_update_in_render.stderr`<br>`tooling/lints/ui/owned_string_into_shared.stderr` |
| ZUP-068 | `1a99eba1926a2776cfb39be3dca922cf08483af7` — Do not redownload same Nightly updates over and over (#59994) | M `crates/auto_update/src/auto_update.rs`<br>M `crates/title_bar/src/update_version.rs` | `actions!(`<br>`pub enum AutoUpdateStatus {`<br>`impl AutoUpdater {`<br>`mod tests {`<br>`use anyhow::anyhow;`<br>`impl UpdateVersion {`<br>`impl Render for UpdateVersion {` | No dedicated upstream test/fixture file changed. |
| ZUP-069 | `d97bbf33699a736c88d2e0ca130fbfa1ea2e9d0a` — agent_ui: Use a callout for the sandbox warning (#60386) | M `crates/agent_ui/src/conversation_view/thread_view.rs` | `impl ThreadView {` | No dedicated upstream test/fixture file changed. |
| ZUP-070 | `e3b73c6b30cdc09e820823fe44542b89850d4be1` — bedrock: Fix Claude Sonnet 5 and Fable 5 routing outside US regions (#60378) | M `crates/bedrock/src/models.rs` | `impl Model {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-071 | `6e9ff7a4f31ad4baa467058765f5fee00e4c2bc7` — Fix agent hyperlinks do not open files (#56283) | M `crates/acp_thread/src/mention.rs`<br>M `crates/agent/src/tools/find_path_tool.rs`<br>M `crates/agent/src/tools/grep_tool.rs`<br>M `crates/agent_ui/src/agent_ui.rs`<br>M `crates/agent_ui/src/conversation_view/thread_view.rs`<br>M `crates/agent_ui/src/ui/mention_crease.rs` | `impl MentionUri {`<br>`impl fmt::Display for MentionLink<'_> {`<br>`mod tests {`<br>`use crate::{AgentTool, ToolCallEventStream, ToolInput};`<br>`impl AgentTool for FindPathTool {`<br>`impl AgentTool for GrepTool {`<br>`use std::path::{Path, PathBuf};`<br>`pub(crate) fn resolve_agent_image(` | No dedicated upstream test/fixture file changed. |
| ZUP-072 | `5b805ac0743660a7034c6504b49a9f10f65524c0` — git_graph: Selectable commit message in detail panel (#59674) | M `crates/git_ui/src/git_graph.rs` | `use gpui::{`<br>`use language::line_diff;`<br>`use ui::{`<br>`struct GitGraphContextMenu {`<br>`pub struct GitGraph {`<br>`impl GitGraph {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-073 | `b4d9194fce5da4d23a558684adc045c8dcf9a620` — themes: Don't try to call `fs.load_bytes` on a directory path on rescans (#60399) | M `crates/zed/src/main.rs` | `fn watch_themes(fs: Arc<dyn fs::Fs>, cx: &mut App) {` | No dedicated upstream test/fixture file changed. |
| ZUP-074 | `04de6dab7c890d815e316f2f9554f9eeeaf8ebb2` — Show type-changed files in commit diffs (#60422) | M `crates/git/src/commit.rs`<br>M `crates/git/src/repository.rs` | `pub fn parse_git_diff_name_status(content: &str) -> impl Iterator<Item = (&str,`<br>`mod tests {`<br>`impl GitRepository for RealGitRepository {` | No dedicated upstream test/fixture file changed. |
| ZUP-075 | `24c5b37e6e4952faf3145f10a97b3806aabfcb17` — Filter AI keybindings when AI features are disabled (#56936) | M `crates/zed/src/zed.rs` | `pub fn handle_keymap_file_changes(`<br>`fn reload_keymaps(cx: &mut App, mut user_key_bindings: Vec<KeyBinding>) {`<br>`pub fn load_default_keymap(cx: &mut App) {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-076 | `62477092a20938d07de278555b1e99fd07b79b4f` — git_panel: Scope folder expansion state to sections (#60396) | M `crates/git_ui/src/git_panel.rs` | `struct TreeViewState {`<br>`impl TreeViewState {`<br>`pub struct GitPanel {`<br>`impl GitPanel {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-077 | `01568e5569b4952b721af8b545173d44c029baaa` — Swap Kotlin LSPs in documentation to reflect actual default config (#54061) | M `docs/src/languages/kotlin.md` | `Report issues to: [https://github.com/zed-extensions/kotlin/issues](https://gith`<br>`under `class Configuration` and initialization_options under `class Initializati`<br>`The following example changes the JVM target from `default` (which is 1.8) to` | No dedicated upstream test/fixture file changed. |
| ZUP-078 | `69664ab9d33590a524df1164a74767c4285f6086` — agent: Show errors when provider is not authenticated but model is configured (#60417) | M `crates/agent/src/agent.rs`<br>M `crates/agent/src/thread.rs`<br>M `crates/agent_ui/src/agent_ui.rs`<br>M `crates/agent_ui/src/conversation_view/thread_view.rs`<br>M `crates/language_model/src/registry.rs`<br>M `crates/language_models/src/language_models.rs` | `impl LanguageModels {`<br>`use gpui::{`<br>`enum CompletionError {`<br>`impl Thread {`<br>`impl EventEmitter<TitleUpdated> for Thread {}`<br>`fn init_language_model_settings(cx: &mut App) {`<br>`fn update_active_language_model_from_settings(cx: &mut App) {`<br>`impl ThreadView {` | No dedicated upstream test/fixture file changed. |
| ZUP-079 | `c4a9b1aa4bb64497b4eef84fb7f2c5988bd6c53b` — Fix blame hover popover not showing on first trigger when inline blame is disabled (#50769) | M `crates/editor/src/editor.rs`<br>M `crates/editor/src/git.rs`<br>M `crates/editor/src/git/blame.rs` | `pub struct Editor {`<br>`impl Editor {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-080 | `159246f0083787124160620118ac35df5f0f3754` — acp: Support boolean ACP config options (#60446) | M `crates/agent_servers/src/acp.rs`<br>M `crates/agent_ui/src/config_options.rs` | `fn client_capabilities_for_agent(`<br>`impl AcpConnection {`<br>`mod tests {`<br>`use collections::HashSet;`<br>`impl ConfigOptionsView {`<br>`impl Render for ConfigOptionSelector {`<br>`fn setting_value_for_config_option_value(` | No dedicated upstream test/fixture file changed. |
| ZUP-081 | `c545fb67d0ce13e335bff76f7c08986000333f2c` — docs: Add Tailwind CSS LSP configuration for Gleam (#58115) | M `docs/src/languages/gleam.md`<br>M `docs/src/languages/tailwindcss.md` | `Gleam support is available through the [Gleam extension](https://github.com/glea`<br>`Languages which can be used with Tailwind CSS in Zed:` | No dedicated upstream test/fixture file changed. |
| ZUP-082 | `0d789ded0ebdeaab69da212f7e0bf1c49d41131d` — extension_rollout: Allow rollout for `extension-workflows` tag (#60354) | M `.github/workflows/extension_workflow_rollout.yml`<br>M `tooling/xtask/src/tasks/workflows/extension_workflow_rollout.rs` | `jobs:`<br>`use serde_json::json;`<br>`use crate::tasks::workflows::{`<br>`fn fetch_extension_repos(filter_repos_input: &WorkflowInput) -> (NamedJob, JobOu`<br>`fn create_rollout_tag(rollout_job: &NamedJob, filter_repos_input: &WorkflowInput` | No dedicated upstream test/fixture file changed. |
| ZUP-083 | `54bf918329ba3bf4ccebee8ea98e0acc0291e201` — acp: Set agent as default when installing it from registry (#60452) | M `crates/agent_ui/src/agent_panel.rs`<br>M `crates/agent_ui/src/agent_registry_ui.rs`<br>M `crates/onboarding/src/basics_page.rs`<br>M `crates/zed_actions/src/lib.rs` | `use zed_actions::{`<br>`pub fn init(cx: &mut App) {`<br>`impl AgentPanel {`<br>`mod tests {`<br>`impl AgentRegistryPage {`<br>`fn render_registry_agent_button(`<br>`pub mod agent {` | No dedicated upstream test/fixture file changed. |
| ZUP-084 | `b1f456390942873767fcc16befdc638b550b9c1b` — docs: Add Tailwind LSP configuration section for Go (Templ) (#55255) | M `docs/src/languages/go.md`<br>M `docs/src/languages/tailwindcss.md` | `In such case Zed won't spawn a new instance of Delve, as it opts to use an exist`<br>`Languages which can be used with Tailwind CSS in Zed:` | No dedicated upstream test/fixture file changed. |
| ZUP-085 | `050e6f3407e0a003ca45a572b8d4132cef5b6dfd` — text_finder: Fix crash when dismissing the finder (#60437) | M `crates/search/src/text_finder.rs` | `pub struct TextFinder {`<br>`impl TextFinder {`<br>`impl ModalView for TextFinder {`<br>`pub struct SearchMatch {` | No dedicated upstream test/fixture file changed. |
| ZUP-086 | `2b0a83ea817d8fd046a3fe4ef92b229e442ab8c6` — picker: Give delegate the initial preview layout (#60007) | M `crates/picker/src/picker.rs` | `impl<D: PickerDelegate> Picker<D> {` | No dedicated upstream test/fixture file changed. |
| ZUP-087 | `e58a02d97409c2ecdbeaee7a936131ccc8f9dc16` — Fix text finder crash from unbounded memory use (#60377) | M `crates/project/src/project_search.rs`<br>M `crates/search/src/text_finder.rs`<br>M `crates/search/src/text_finder/delegate.rs` | `impl Search {`<br>`pub struct SearchMatch {`<br>`use picker::{Picker, PickerDelegate};`<br>`fn multibuffer_ranges_to_search_matches<'a>(`<br>`impl Delegate {`<br>`const SEARCH_RESULTS_BATCH_SIZE: usize = 256;`<br>`impl PickerDelegate for Delegate {`<br>`async fn stream_results_to_picker(` | No dedicated upstream test/fixture file changed. |
| ZUP-088 | `f16b46419dc84b357afe2ed0cd187440de6e9c7a` — Improve diff tabs toolbar design (#60464) | M `assets/icons/square_dot.svg`<br>M `assets/icons/square_minus.svg`<br>M `assets/icons/square_plus.svg`<br>M `crates/agent_ui/src/agent_diff.rs`<br>M `crates/agent_ui/src/conversation_view/thread_view.rs`<br>M `crates/editor/src/editor.rs`<br>M `crates/editor/src/element.rs`<br>M `crates/editor/src/element/header.rs`<br>M `crates/editor/src/split.rs`<br>M `crates/git_ui/src/commit_modal.rs`<br>M `crates/git_ui/src/git_panel.rs`<br>M `crates/git_ui/src/git_ui.rs`<br>M `crates/git_ui/src/project_diff.rs`<br>M `crates/git_ui/src/solo_diff_view.rs`<br>M `crates/picker/src/footer.rs`<br>M `crates/search/src/buffer_search.rs`<br>M `crates/ui/src/components/button/split_button.rs`<br>M `crates/ui/src/components/divider.rs` | `-<path d="M12.6667 2H3.33333C2.59695 2 2 2.59695 2 3.33333V12.6667C2 13.403 2.59695 14 3.33333 14H12.6667C13.403 14 14 13.403 14 12.6667V3.33333C14 2.59695 13.403 2 12.6667 2Z" stroke="black" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>`<br>`use std::{`<br>`impl Render for AgentDiffToolbar {`<br>`impl ThreadView {`<br>`pub use element::{`<br>`pub use multi_buffer::{`<br>`pub(crate) use header::StickyHeader;`<br>`use ui::{` | No dedicated upstream test/fixture file changed. |
| ZUP-089 | `5d6c88cdb71168d31c35a1057702f2fa26d5494e` — workspace: Use ellipsis character in "New" tooltip (#60467) | M `crates/workspace/src/pane.rs` | `fn default_render_tab_bar_buttons(` | No dedicated upstream test/fixture file changed. |
| ZUP-090 | `283d5054ec5663795e8f9d744260c6c57b85f9e3` — ci: Set and enforce more default permissions (#60222) | M `.github/workflows/after_release.yml`<br>M `.github/workflows/autofix_pr.yml`<br>M `.github/workflows/bump_collab_staging.yml`<br>M `.github/workflows/bump_patch_version.yml`<br>M `.github/workflows/bump_zed_version.yml`<br>M `.github/workflows/cherry_pick.yml`<br>M `.github/workflows/comment_on_potential_duplicate_issues.yml`<br>M `.github/workflows/community_close_stale_issues.yml`<br>M `.github/workflows/community_update_all_top_ranking_issues.yml`<br>M `.github/workflows/community_update_weekly_top_ranking_issues.yml`<br>M `.github/workflows/compliance_check.yml`<br>M `.github/workflows/congrats.yml`<br>M `.github/workflows/danger.yml`<br>M `.github/workflows/deploy_collab.yml`<br>M `.github/workflows/deploy_docs.yml`<br>M `.github/workflows/deploy_nightly_docs.yml`<br>M `.github/workflows/docs_suggestions.yml`<br>M `.github/workflows/extension_auto_bump.yml`<br>M `.github/workflows/extension_bump.yml`<br>M `.github/workflows/extension_tests.yml`<br>M `.github/workflows/extension_workflow_rollout.yml`<br>M `.github/workflows/good_first_issue_notifier.yml`<br>M `.github/workflows/hotfix-review-monitor.yml`<br>M `.github/workflows/nix_build.yml`<br>M `.github/workflows/publish_extension_cli.yml`<br>M `.github/workflows/release.yml`<br>M `.github/workflows/release_nightly.yml`<br>M `.github/workflows/run_bundling.yml`<br>M `.github/workflows/run_tests.yml`<br>M `.github/workflows/slack_notify_community_automation_failure.yml`<br>M `.github/workflows/slack_notify_first_responders.yml`<br>M `.github/workflows/slack_notify_label_created.yml`<br>M `.github/workflows/stale-pr-reminder.yml`<br>M `.github/workflows/update_duplicate_magnets.yml`<br>M `extensions/workflows/run_tests.yml`<br>M `extensions/workflows/shared/bump_version.yml`<br>M `tooling/xtask/src/tasks/workflow_checks.rs`<br>A `tooling/xtask/src/tasks/workflow_checks/check_permissions.rs`<br>M `tooling/xtask/src/tasks/workflow_checks/check_run_patterns.rs`<br>M `tooling/xtask/src/tasks/workflows.rs`<br>M `tooling/xtask/src/tasks/workflows/after_release.rs`<br>M `tooling/xtask/src/tasks/workflows/autofix_pr.rs`<br>M `tooling/xtask/src/tasks/workflows/bump_patch_version.rs`<br>M `tooling/xtask/src/tasks/workflows/bump_zed_version.rs`<br>M `tooling/xtask/src/tasks/workflows/cherry_pick.rs`<br>M `tooling/xtask/src/tasks/workflows/compliance_check.rs`<br>M `tooling/xtask/src/tasks/workflows/danger.rs`<br>M `tooling/xtask/src/tasks/workflows/deploy_collab.rs`<br>M `tooling/xtask/src/tasks/workflows/deploy_docs.rs`<br>M `tooling/xtask/src/tasks/workflows/extension_auto_bump.rs`<br>M `tooling/xtask/src/tasks/workflows/extension_bump.rs`<br>M `tooling/xtask/src/tasks/workflows/extension_tests.rs`<br>M `tooling/xtask/src/tasks/workflows/extension_workflow_rollout.rs`<br>M `tooling/xtask/src/tasks/workflows/extensions/bump_version.rs`<br>M `tooling/xtask/src/tasks/workflows/extensions/run_tests.rs`<br>M `tooling/xtask/src/tasks/workflows/nix_build.rs`<br>M `tooling/xtask/src/tasks/workflows/publish_extension_cli.rs`<br>M `tooling/xtask/src/tasks/workflows/release.rs`<br>M `tooling/xtask/src/tasks/workflows/release_nightly.rs`<br>M `tooling/xtask/src/tasks/workflows/run_bundling.rs`<br>M `tooling/xtask/src/tasks/workflows/run_tests.rs`<br>M `tooling/xtask/src/tasks/workflows/steps.rs` | `on:`<br>`jobs:`<br>`concurrency:`<br>`permissions:`<br>`+mod check_permissions;`<br>`mod check_run_patterns;`<br>`use std::{fs, path::PathBuf};`<br>`use strum::IntoEnumIterator;` | `.github/workflows/extension_tests.yml`<br>`.github/workflows/run_tests.yml`<br>`extensions/workflows/run_tests.yml`<br>`tooling/xtask/src/tasks/workflows/extension_tests.rs`<br>`tooling/xtask/src/tasks/workflows/extensions/run_tests.rs`<br>`tooling/xtask/src/tasks/workflows/run_tests.rs` |
| ZUP-091 | `eeff97950f7ccfd5b2f73b48f7267bd0df5e4bfb` — Add license to tooling/lints crate (#60468) | M `.github/workflows/run_tests.yml`<br>A `tooling/lints/LICENSE-APACHE`<br>M `tooling/xtask/src/tasks/workflows/run_tests.rs` | `jobs:`<br>`+../../LICENSE-APACHE`<br>`fn orchestrate_impl(rules: &[&PathCondition], target: OrchestrateTarget) -> Name` | `.github/workflows/run_tests.yml`<br>`tooling/xtask/src/tasks/workflows/run_tests.rs` |
| ZUP-092 | `48c03b2f7f2a9ca8e3ee0fd6e1c4eba097d9f516` — Better "Restart to Update" button dismissals (#60448) | M `crates/auto_update/src/auto_update.rs`<br>M `crates/title_bar/src/update_version.rs` | `pub struct AutoUpdater {`<br>`impl AutoUpdater {`<br>`pub struct UpdateVersion {`<br>`impl UpdateVersion {`<br>`impl Render for UpdateVersion {` | No dedicated upstream test/fixture file changed. |
| ZUP-093 | `98fe76caadfd71d09eb29de885d0e7b439956e38` — editor: Fix completion labels not being rendered completely (#56976) | M `crates/editor/src/code_context_menus.rs` | `use gpui::{`<br>`impl CompletionsMenu {`<br>`fn completion_kind_highlight_name(kind: CompletionItemKind) -> Option<&'static s`<br>`impl CodeActionsMenu {` | No dedicated upstream test/fixture file changed. |
| ZUP-094 | `9064f26a454f8aa32c126d8c798e1063f3f95210` — cloud_api_types: Add new ID fields to `AuthenticatedUser` (#60497) | M `crates/client/src/test.rs`<br>M `crates/cloud_api_types/src/cloud_api_types.rs`<br>M `crates/collab/src/auth.rs` | `pub fn make_get_authenticated_user_response(`<br>`pub struct AuthenticatedUser {`<br>`pub async fn validate_header<B>(mut req: Request<B>, next: Next<B>) -> impl Into` | `crates/client/src/test.rs` |
| ZUP-095 | `e7311d52ba1b7ec8f2c1651e32bd78e0da4cbca9` — Split interpolate failure rejection reason (#60499) | M `crates/cloud_llm_client/src/cloud_llm_client.rs`<br>M `crates/codestral/src/codestral.rs`<br>M `crates/edit_prediction/src/edit_prediction_tests.rs`<br>M `crates/edit_prediction/src/prediction.rs`<br>M `crates/edit_prediction/src/zed_edit_prediction_delegate.rs`<br>M `crates/edit_prediction_types/src/edit_prediction_types.rs` | `pub enum EditPredictionRejectReason {`<br>`impl CurrentCompletion {`<br>`async fn test_interpolated_empty(cx: &mut TestAppContext) {`<br>`impl EditPredictionResult {`<br>`impl EditPredictionDelegate for ZedEditPredictionDelegate {`<br>`pub fn interpolate_edits(` | `crates/edit_prediction/src/edit_prediction_tests.rs` |
| ZUP-096 | `52bc5a0488d4964a5981cbd923409d55341b088a` — run_tests: Stop treating non-workspace dirs as packages (#60502) | M `.github/workflows/run_tests.yml`<br>M `tooling/xtask/src/tasks/workflows/run_tests.rs` | `jobs:`<br>`fn orchestrate_impl(rules: &[&PathCondition], target: OrchestrateTarget) -> Name` | `.github/workflows/run_tests.yml`<br>`tooling/xtask/src/tasks/workflows/run_tests.rs` |
| ZUP-097 | `872ca8fef52fe527fc922e8bf61e93201de79878` — Add license symlinks to lint test fixture crates (#60505) | A `tooling/lints/test_fixture/LICENSE-APACHE`<br>A `tooling/lints/test_fixture/consumer/LICENSE-APACHE`<br>A `tooling/lints/test_fixture/gpui/LICENSE-APACHE`<br>A `tooling/lints/test_fixture/gpui_shared_string/LICENSE-APACHE`<br>A `tooling/lints/test_fixture/render_consumer/LICENSE-APACHE` | `+../../../LICENSE-APACHE`<br>`+../../../../LICENSE-APACHE` | No dedicated upstream test/fixture file changed. |
| ZUP-098 | `c31b2b0dc7180247b2981eb084594efaf11ee396` — Git partially staged changes (#46541) | M `crates/agent_ui/src/agent_diff.rs`<br>M `crates/agent_ui/src/entry_view_state.rs`<br>M `crates/buffer_diff/src/buffer_diff.rs`<br>M `crates/editor/src/config.rs`<br>M `crates/editor/src/editor.rs`<br>M `crates/editor/src/element.rs`<br>M `crates/editor/src/git.rs`<br>M `crates/editor/src/split.rs`<br>A `crates/git_ui/src/branch_diff.rs`<br>M `crates/git_ui/src/commit_view.rs`<br>M `crates/git_ui/src/conflict_view.rs`<br>A `crates/git_ui/src/diff_multibuffer.rs`<br>M `crates/git_ui/src/file_diff_view.rs`<br>M `crates/git_ui/src/git_panel.rs`<br>M `crates/git_ui/src/git_ui.rs`<br>M `crates/git_ui/src/multi_diff_view.rs`<br>M `crates/git_ui/src/project_diff.rs`<br>A `crates/git_ui/src/staged_diff.rs`<br>M `crates/git_ui/src/text_diff_view.rs`<br>A `crates/git_ui/src/unstaged_diff.rs`<br>M `crates/multi_buffer/src/multi_buffer.rs`<br>M `crates/project/src/git_store.rs`<br>R076 `crates/project/src/git_store/branch_diff.rs` → `crates/project/src/git_store/diff_buffer_list.rs`<br>M `crates/project/src/project.rs`<br>M `crates/project/tests/integration/project_tests.rs`<br>M `crates/proto/proto/git.proto`<br>M `crates/search/src/buffer_search.rs`<br>M `crates/zed/src/zed.rs`<br>M `crates/zed_actions/src/lib.rs` | `use editor::{`<br>`impl AgentDiffPane {`<br>`impl Render for AgentDiffPane {`<br>`fn diff_hunk_controls(`<br>`impl AgentDiff {`<br>`-use std::ops::Range;`<br>`use collections::{HashMap, HashSet};`<br>`fn create_editor_diff(` | `crates/project/tests/integration/project_tests.rs` |
| ZUP-099 | `28c2e7d1e4dc9c8df965adf7656f9b1d993c9a23` — Fix tree-sitter-markdown scanner serialize buffer overflow (#60312) | M `Cargo.lock`<br>M `Cargo.toml` | `version = "0.3.2"`<br>`tree-sitter-json = "0.24"` | No dedicated upstream test/fixture file changed. |
| ZUP-100 | `d9ada8487bca20be32c7621e296030c9d68362f5` — Fix hard-tab block autoindent skipping unindented lines (#60406) | M `crates/language/src/buffer.rs`<br>M `crates/language/src/buffer_tests.rs` | `impl Buffer {`<br>`fn test_autoindent_block_mode_without_original_indent_columns(cx: &mut App) {` | `crates/language/src/buffer_tests.rs` |
| ZUP-101 | `001bda1a46af8a8e7d0abc89aaf67a6a1193426f` — project: Fix ref-match rust completions dropping the leading & (#60521) | M `crates/editor/src/editor_tests.rs`<br>M `crates/project/src/lsp_store.rs` | `async fn test_completions_with_additional_edits(cx: &mut TestAppContext) {`<br>`impl LspStore {` | `crates/editor/src/editor_tests.rs` |
| ZUP-102 | `a956add0b655f5b20665cf34641f7c08ede0dafc` — Fix MCP tools with $ref/$defs being silently rejected (#60165) | M `Cargo.lock`<br>M `crates/language_model_core/Cargo.toml`<br>M `crates/language_model_core/src/tool_schema.rs` | `dependencies = [`<br>`thiserror.workspace = true`<br>`pub fn adapt_schema_to_format(`<br>`fn preprocess_json_schema(json: &mut Value) -> Result<()> {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-103 | `fcd0f769521884e71b9b54ab964bd88cf057a276` — git_ui: make git graph columns toggleable (#59850) | M `crates/git_ui/src/git_graph.rs`<br>M `crates/ui/src/components/context_menu.rs`<br>M `crates/ui/src/components/data_table.rs`<br>M `crates/ui/src/components/data_table/table_row.rs`<br>M `crates/ui/src/components/data_table/tests.rs`<br>M `crates/ui/src/components/redistributable_columns.rs` | `use ui::{`<br>`const CUSTOM_GIT_COMMANDS_DOCS_SLUG: &str = "tasks#custom-git-commands";`<br>`struct GitGraphContextMenu {`<br>`pub struct GitGraph {`<br>`impl GitGraph {`<br>`impl Render for GitGraph {`<br>`impl workspace::SerializableItem for GitGraph {`<br>`mod persistence {` | `crates/ui/src/components/data_table/tests.rs` |
| ZUP-104 | `a29c0d41f4787d7cd268d4f8ad1a706af86042c7` — gpui: Add input region support for Wayland windows (#60161) | M `crates/gpui/src/platform.rs`<br>M `crates/gpui/src/window.rs`<br>M `crates/gpui_linux/src/linux/wayland/window.rs` | `pub trait PlatformWindow: HasWindowHandle + HasDisplayHandle {`<br>`impl Window {`<br>`impl PlatformWindow for WaylandWindow {` | No dedicated upstream test/fixture file changed. |
| ZUP-105 | `e7803a88f55e290a0478d46bd0137e711d6da862` — gpui: Add ParentElement impl for AnimationElement (#54145) | M `crates/gpui/src/elements/animation.rs` | `use crate::{`<br>`pub struct AnimationElement<E> {`<br>`mod easing {` | No dedicated upstream test/fixture file changed. |
| ZUP-106 | `20a93f6195ca8e9f0a748038317a5efe1be3e482` — gpui_macos: Fix glyph rendering when fonts share a PostScript name (#57250) | M `crates/gpui_macos/src/text_system.rs` | `use cocoa::appkit::CGFloat;`<br>`impl MacTextSystemState {` | No dedicated upstream test/fixture file changed. |
| ZUP-107 | `7194f987a74317881768b4d3a5ce1cc93a3228bd` — Fix disabled Windows window controls (#60440) | M `crates/gpui_windows/src/events.rs`<br>M `crates/gpui_windows/src/window.rs` | `impl WindowsWindowInner {`<br>`pub(crate) struct WindowsWindowInner {`<br>`struct WindowCreateContext {`<br>`impl WindowsWindow {` | No dedicated upstream test/fixture file changed. |
| ZUP-108 | `71b3b9b8874218170de42cc98f96816b683b2005` — gpui_macos: Guard start_display_link against nil screens (#60419) | M `crates/gpui_macos/src/window.rs` | `impl MacWindowState {`<br>`impl MacWindow {`<br>`where`<br>`unsafe fn display_id_for_screen(screen: id) -> CGDirectDisplayID {`<br>`extern "C" fn toggle_tab_bar(this: &Object, _sel: Sel, _id: id) {` | No dedicated upstream test/fixture file changed. |
| ZUP-109 | `811fe501a77ffb82c1c1b3d9934ec4e416f853e0` — workspace: Fix missing icons on non-terminal tabs during drag (#53637) | M `crates/workspace/src/pane.rs` | `impl Pane {`<br>`impl Render for DraggedTab {` | No dedicated upstream test/fixture file changed. |
| ZUP-110 | `64c55b038f1e676c11bc4a2fdfbc8117f03144ca` — Change OpenAI compatible URL placeholder in edit prediction settings (#58771) | M `crates/settings_ui/src/pages/edit_prediction_provider_setup.rs` | `const OLLAMA_MODEL_PLACEHOLDER: &str = "qwen2.5-coder:3b-base";`<br>`fn open_ai_compatible_settings() -> Box<[SettingsPageItem]> {` | No dedicated upstream test/fixture file changed. |
| ZUP-111 | `452e1cb27365fbaa935eba1b2aca98279350cbaa` — opencode: Model updates + fixes (#60526) | M `crates/language_models/src/provider/opencode.rs`<br>M `crates/opencode/src/opencode.rs`<br>M `crates/settings_content/src/language_model.rs`<br>M `docs/src/ai/use-api-access.md` | `use opencode::{ApiProtocol, OPENCODE_API_URL, OpenCodeSubscription};`<br>`impl LanguageModelProvider for OpenCodeLanguageModelProvider {`<br>`impl LanguageModel for OpenCodeLanguageModel {`<br>`pub enum Model {`<br>`impl Model {`<br>`pub struct OpenCodeSettingsContent {`<br>`pub struct OpenCodeAvailableModel {`<br>`The available configuration options for custom OpenCode models are:` | No dedicated upstream test/fixture file changed. |
| ZUP-112 | `2eeca73e665a814c5b1430a3bfd8aacff0477fb3` — auto_update: Fix installer temp dirs leaking on macOS (#60528) | M `crates/auto_update/src/auto_update.rs` | `struct MacOsUnmounter<'a> {`<br>`impl Drop for MacOsUnmounter<'_> {`<br>`pub fn view_release_notes(_: &ViewReleaseNotes, cx: &mut App) -> Option<()> {`<br>`impl InstallerDir {`<br>`impl AutoUpdater {`<br>`async fn install_release_macos(` | No dedicated upstream test/fixture file changed. |
| ZUP-113 | `59c021d8dcee7f5b0ac17d298dce96ca8633421e` — copilot_ui: Fix Copilot sign-in window focus on Hyprland (#59933) | M `Cargo.lock`<br>M `crates/copilot_ui/Cargo.toml`<br>M `crates/copilot_ui/src/sign_in.rs` | `dependencies = [`<br>`project.workspace = true`<br>`use project::project_settings::ProjectSettings;`<br>`fn open_copilot_code_verification_window(copilot: &Entity<Copilot>, window: &Win` | No dedicated upstream test/fixture file changed. |
| ZUP-114 | `693962917b5a015949ad2e768bc10ea169d41546` — gpui: Fix clear drag overlay when external drag ends outside window (#45759) | M `crates/gpui/src/elements/div.rs`<br>M `crates/project_panel/src/project_panel.rs` | `use crate::{`<br>`impl Interactivity {`<br>`pub trait InteractiveElement: Sized {`<br>`pub(crate) type MouseMoveListener =`<br>`pub struct Interactivity {`<br>`use gpui::{`<br>`impl ProjectPanel {`<br>`impl Render for ProjectPanel {` | No dedicated upstream test/fixture file changed. |
| ZUP-115 | `961f4f202465629fe00ccf55bd9ebf01d643b931` — gpui: Refresh mouse position after bounds changes (#60421) | M `crates/gpui/src/window.rs`<br>M `crates/gpui_linux/src/linux/x11/window.rs` | `impl Window {`<br>`impl PlatformWindow for X11Window {` | No dedicated upstream test/fixture file changed. |
| ZUP-116 | `fbd911ed3e0fdb98ab5ae7a66679774d4986db0a` — Limit SVG Pixmap size to avoid GPUI texture allocation errors (#56468) | M `crates/gpui/src/svg_renderer.rs` | `impl SvgRenderer {` | No dedicated upstream test/fixture file changed. |
| ZUP-117 | `c05b439174c95a51df92a797c2a17933ae44a59e` — acp: Show descriptions for elicitation options (#60527) | M `Cargo.lock`<br>M `Cargo.toml`<br>M `crates/agent_ui/src/conversation_view/elicitation.rs` | `name = "agent-client-protocol"`<br>`source = "registry+https://github.com/rust-lang/crates.io-index"`<br>`name = "agent-client-protocol-derive"`<br>`name = "agent-client-protocol-schema"`<br>`accesskit_windows = "0.33.1"`<br>`struct ElicitationOption {`<br>`mod tests {`<br>`fn preview_form_schema() -> acp::ElicitationSchema {` | No dedicated upstream test/fixture file changed. |
| ZUP-118 | `f360136f199373607c4449414145b3287baa5d85` — project: Create LSP file watcher on the background (#60530) | M `crates/project/src/lsp_store.rs`<br>M `crates/session/src/session.rs` | `impl LanguageServerWatchedPaths {`<br>`impl AppSession {` | No dedicated upstream test/fixture file changed. |
| ZUP-119 | `a94fa5cf1913b0d80dfaf8fa495486091ac79cb7` — git_graph: Add design adjustments (#60469) | M `crates/git_ui/src/git_graph.rs`<br>M `crates/git_ui/src/git_panel.rs`<br>M `crates/git_ui/src/project_diff.rs`<br>M `crates/outline_panel/src/outline_panel.rs`<br>M `crates/project_panel/src/project_panel.rs`<br>M `crates/ui/src/components/data_table.rs`<br>M `crates/ui/src/components/indent_guides.rs` | `use ui::{`<br>`const CUSTOM_GIT_COMMANDS_DOCS_SLUG: &str = "tasks#custom-git-commands";`<br>`const TABLE_COLUMN_COUNT: usize = 4;`<br>`impl ChangedFileEntry {`<br>`impl ChangedFileDirectoryEntry {`<br>`impl GitGraph {`<br>`impl Render for GitGraph {`<br>`use gpui::{` | No dedicated upstream test/fixture file changed. |
| ZUP-120 | `fc827a218e979062b079efbd448947989fb86ab8` — Update issue ranking script dependencies (#60345) | M `script/update_top_ranking_issues/uv.lock` | `name = "idna"`<br>`source = { registry = "https://pypi.org/simple" }`<br>`wheels = [`<br>`name = "pygments"` | No dedicated upstream test/fixture file changed. |
| ZUP-121 | `d564495dde0c344fa54e2432b91680fc604e495c` — git_ui: Show tags in Git panel history (#60534) | M `crates/git_ui/src/git_panel.rs` | `use git::repository::{`<br>`use ui::{`<br>`const TREE_INDENT: f32 = 16.0;`<br>`pub struct GitPanel {`<br>`struct BulkStaging {`<br>`impl GitPanel {` | No dedicated upstream test/fixture file changed. |
| ZUP-122 | `bc29bcfe728e3ce158f36b6a1b15f3dff667fe02` — git: Load buffer git diff bases with a single batched git process (#59357) | M `crates/fs/src/fake_git_repo.rs`<br>M `crates/git/src/repository.rs`<br>M `crates/project/src/git_store.rs` | `impl GitRepository for FakeGitRepository {`<br>`pub trait GitRepository: Send + Sync {`<br>`impl GitRepository for RealGitRepository {`<br>`mod tests {`<br>`impl Repository {` | No dedicated upstream test/fixture file changed. |
| ZUP-123 | `3cbe4c298e78d57b88b2b308951405752d6dc3ad` — Add missing panels to View menu (#60356) | M `crates/git_ui/src/git_panel.rs`<br>M `crates/zed/src/zed/app_menus.rs`<br>M `crates/zed_actions/src/lib.rs` | `use workspace::{`<br>`actions!(`<br>`use terminal_view::terminal_panel;`<br>`pub fn app_menus(cx: &mut App) -> Vec<Menu> {`<br>`pub mod notebook {` | No dedicated upstream test/fixture file changed. |
| ZUP-124 | `5a7d414a23938c5efb674d0c2948813e37448eea` — fs: Skip parent watch for poll watcher symlink targets (#57049) | M `crates/fs/src/fs.rs`<br>M `crates/fs/src/fs_watcher.rs` | `impl Fs for RealFs {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-125 | `56009f39bb916be80baace86919267cf0e09035c` — gpui_wgpu: Fix wgpu renderer teardown panic (#60160) | M `crates/gpui_wgpu/src/wgpu_renderer.rs` | `impl WgpuRenderer {` | No dedicated upstream test/fixture file changed. |
| ZUP-126 | `8d932b0e0671645500b567cd474cb113085638fd` — git_ui: Use pull request link from push output in push toast (#60522) | M `crates/git_ui/src/git_panel.rs`<br>M `crates/git_ui/src/remote_output.rs` | `impl GitPanel {`<br>`use util::ResultExt as _;`<br>`pub enum SuccessStyle {`<br>`pub struct SuccessMessage {`<br>`pub fn format_output(action: &RemoteAction, output: RemoteCommandOutput) -> Succ`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-127 | `5d7d1d3f09f1d0934a472f654a6448a366497edf` — language_extension: Hold LspStore weakly in LspAccess::ViaLspStore (#60558) | M `crates/extension_host/src/extension_store_test.rs`<br>M `crates/language_extension/src/extension_lsp_adapter.rs`<br>M `crates/language_extension/src/language_extension.rs`<br>M `crates/remote_server/src/headless_project.rs` | `async fn test_extension_store_with_test_extension(cx: &mut TestAppContext) {`<br>`impl ExtensionLanguageServerProxy for LanguageServerRegistryProxy {`<br>`use extension::{ExtensionGrammarProxy, ExtensionHostProxy, ExtensionLanguageProx`<br>`pub enum LspAccess {`<br>`impl HeadlessProject {` | `crates/extension_host/src/extension_store_test.rs` |
| ZUP-128 | `d12b980ee0721dc7fb72aeea511b1ab7b62af98c` — bedrock: Add native support for Bedrock Mantle models (#60480) | M `Cargo.lock`<br>M `Cargo.toml`<br>M `crates/bedrock/src/models.rs`<br>M `crates/language_models/Cargo.toml`<br>M `crates/language_models/src/provider/bedrock.rs`<br>M `crates/language_models/src/settings.rs`<br>M `crates/open_ai/src/completion.rs`<br>M `crates/open_ai/src/responses.rs`<br>M `crates/settings_content/src/language_model.rs`<br>M `docs/src/ai/use-a-gateway.md` | `dependencies = [`<br>`aws-sdk-bedrockruntime = { version = "1.112.0", features = [`<br>`pub struct BedrockModelCacheConfiguration {`<br>`pub enum Model {`<br>`impl Model {`<br>`mod tests {`<br>`aws-credential-types = { workspace = true, features = ["hardcoded-credentials"]`<br>`use aws_config::{BehaviorVersion, Region};` | No dedicated upstream test/fixture file changed. |
| ZUP-129 | `f9c994796ad4341649d7b8664edbdfaae8bebd5d` — cloud_api_client: Make `send_authenticated_json_request` public (#60562) | M `crates/cloud_api_client/src/cloud_api_client.rs` | `impl CloudApiClient {` | No dedicated upstream test/fixture file changed. |
| ZUP-130 | `950ec7943f3f1c0532caf6b91d818fb6349c8927` — editor: Decode escaped characters in hover popover links (#55973) | M `Cargo.lock`<br>M `crates/editor/Cargo.toml`<br>M `crates/editor/src/hover_popover.rs` | `dependencies = [`<br>`url.workspace = true`<br>`pub fn diagnostics_markdown_style(window: &Window, cx: &App) -> MarkdownStyle {`<br>`pub fn open_markdown_url(`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-131 | `d4dfe87ce064d1d8eb13404decd96afe4552478b` — workspace: Skip closed items that cannot be reopened (#56299) | M `crates/workspace/src/workspace.rs` | `impl Workspace {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-132 | `01235732e683e524349585c06f290a4803806c48` — Fix should_log_lsp_request_failure logic inversion (#57554) | M `crates/project/src/lsp_store.rs` | `fn should_log_lsp_request_failure(message: &str) -> bool {`<br>`fn extend_formatting_transaction(` | No dedicated upstream test/fixture file changed. |
| ZUP-133 | `bc3c9422f4b29cdafd64fc89853b8705a359a0e7` — Use more .array_windows::<N>() (#58877) | M `crates/editor/src/input.rs`<br>M `crates/file_finder/src/file_finder.rs`<br>M `crates/markdown/src/html/html_rendering.rs`<br>M `crates/outline_panel/src/outline_panel.rs`<br>M `crates/zeta_prompt/src/multi_region.rs` | `impl Editor {`<br>`impl<'a> PathComponentSlice<'a> {`<br>`impl MarkdownElement {`<br>`impl OutlinePanel {`<br>`fn cursor_block_index(cursor_offset: Option<usize>, marker_offsets: &[usize]) ->`<br>`hhhhhhhhhh = 8;` | No dedicated upstream test/fixture file changed. |
| ZUP-134 | `2ab35c6b7d594288cdfee6695d1afa8f7ce91444` — Consistently use `context()` to preserve sources for anyhow errors (#59112) | M `crates/agent/src/db.rs`<br>M `crates/keymap_editor/src/keymap_editor.rs`<br>M `crates/remote/src/transport/ssh.rs`<br>M `crates/remote/src/transport/wsl.rs` | `use agent_settings::AgentProfileId;`<br>`impl ThreadsDatabase {`<br>`async fn save_keybinding_update(`<br>`impl RemoteConnection for SshRemoteConnection {`<br>`impl WslRemoteConnection {`<br>`impl RemoteConnection for WslRemoteConnection {` | No dedicated upstream test/fixture file changed. |
| ZUP-135 | `2243c13b9b224311fdb64362b2310c364f7dfbbb` — project: Fix content swap when an LSP rename also renames the file (#59104) | M `crates/project/src/lsp_store.rs`<br>M `crates/project/tests/integration/project_tests.rs` | `impl LocalLspStore {`<br>`async fn test_rename(cx: &mut gpui::TestAppContext) {` | `crates/project/tests/integration/project_tests.rs` |
| ZUP-136 | `35ddcb2ecd07d38b1579c5199f175d0e9c2133f4` — go: Fix outline for methods with unnamed receivers (#58656) | M `crates/grammars/src/go/outline.scm`<br>M `crates/languages/src/go.rs` | `-      name: (_) @context`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-137 | `385e9c68ae4a068361417ec1aa55e5f727f615fa` — Suggest language extensions for untitled buffers (#55263) | M `crates/acp_thread/src/diff.rs`<br>M `crates/agent_ui/src/diagnostics.rs`<br>M `crates/edit_prediction_ui/src/rate_prediction_modal.rs`<br>M `crates/editor/src/edit_prediction.rs`<br>M `crates/editor/src/editor.rs`<br>M `crates/editor/src/element/header.rs`<br>M `crates/editor/src/items.rs`<br>M `crates/git_ui/src/file_diff_view.rs`<br>M `crates/git_ui/src/multi_diff_view.rs`<br>M `crates/git_ui/src/text_diff_view.rs`<br>M `crates/multi_buffer/src/multi_buffer.rs` | `impl Diff {`<br>`impl PendingDiff {`<br>`use language::{Anchor, BufferSnapshot, DiagnosticEntryRef, DiagnosticSeverity, T`<br>`pub fn codeblock_fence_for_path(`<br>`impl RatePredictionsModal {`<br>`impl Editor {`<br>`use language::language_settings::ShowWhitespaceSetting;`<br>`pub(crate) fn render_buffer_header(` | No dedicated upstream test/fixture file changed. |
| ZUP-138 | `ded93ccb085d9271c7820abc8c39b9a0a04b5d16` — Fix typos and grammatical mistakes in docs (#59495) | M `docs/src/ai/edit-prediction.md`<br>M `docs/src/ai/mcp.md`<br>M `docs/src/ai/parallel-agents.md`<br>M `docs/src/authentication.md`<br>M `docs/src/command-palette.md`<br>M `docs/src/configuring-languages.md`<br>M `docs/src/debugger.md`<br>M `docs/src/development/glossary.md`<br>M `docs/src/diagnostics.md`<br>M `docs/src/environment.md`<br>M `docs/src/extensions/debugger-extensions.md`<br>M `docs/src/extensions/developing-extensions.md`<br>M `docs/src/git.md`<br>M `docs/src/languages/ansible.md`<br>M `docs/src/languages/cpp.md`<br>M `docs/src/languages/elixir.md`<br>M `docs/src/languages/java.md`<br>M `docs/src/languages/javascript.md`<br>M `docs/src/languages/json.md`<br>M `docs/src/languages/powershell.md`<br>M `docs/src/languages/python.md`<br>M `docs/src/languages/rust.md`<br>M `docs/src/languages/typescript.md`<br>M `docs/src/linux.md`<br>M `docs/src/performance.md`<br>M `docs/src/project-panel.md`<br>M `docs/src/reference/all-settings.md`<br>M `docs/src/remote-development.md`<br>M `docs/src/repl.md`<br>M `docs/src/vim.md`<br>M `docs/src/worktree-trust.md` | `Edit Prediction has two display modes:`<br>`Most MCP servers require configuration after installation.`<br>`Once you're in a new worktree, use the branch picker next to the worktree picker`<br>`Stripe is used for billing, and will use your Zed account's email address when s`<br>`To try it, open the Command Palette and type `new file`. The command list should`<br>`Here's how you would structure these settings in Zed's `settings.json`:`<br>`Populate this file with the same array of objects you would place in `.zed/debug`<br>`h_flex()` | No dedicated upstream test/fixture file changed. |
| ZUP-139 | `739f23926d58a5c4dbda736e5b3b2ebd943337de` — grammars: Recognize Gentoo `ebuild` files as bash script (#59068) | M `crates/edit_prediction_cli/src/filter_languages.rs`<br>M `crates/grammars/src/bash/config.toml` | `mod tests {`<br>`grammar = "bash"` | No dedicated upstream test/fixture file changed. |
| ZUP-140 | `f5c975162cf217f2c9cd1a2c1192eb2bb4653cdc` — terminal: Open links with Cmd/Ctrl-click when mouse mode is enabled (#60067) | M `assets/settings/default.json`<br>M `crates/settings/src/vscode_import.rs`<br>M `crates/settings_content/src/terminal.rs`<br>M `crates/settings_ui/src/page_data.rs`<br>M `crates/terminal/src/terminal.rs`<br>M `crates/terminal/src/terminal_settings.rs`<br>M `docs/src/reference/all-settings.md`<br>M `docs/src/terminal.md` | `+    // Whether cmd-click (ctrl-click on Linux and Windows) opens hyperlinks even`<br>`impl VsCodeSettings {`<br>`pub struct TerminalSettingsContent {`<br>`fn terminal_page() -> SettingsPage {`<br>`impl TerminalBuilder {`<br>`pub struct Terminal {`<br>`impl Terminal {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-141 | `a3a4719a8a246e2f4b88a1fdb0d94809726cad5f` — project_panel: Open files in a permanent tab on middle click (#60563) | M `crates/project_panel/src/project_panel.rs`<br>M `docs/src/project-panel.md` | `impl ProjectPanel {`<br>`permanent tab. Editing the file or double-clicking it promotes it to a permanent` | No dedicated upstream test/fixture file changed. |
| ZUP-142 | `05530c9b354d487bda520a3c74bb02e90988b2fd` — acp_thread: Log error when checkpoint comparison fails (#59196) | M `crates/acp_thread/src/acp_thread.rs` | `impl AcpThread {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-143 | `86b2831c7c2e210ce150b2771c89dc6bd70ae8a7` — editor: Use raw head selection for rename (#60594) | M `crates/editor/src/editor.rs`<br>M `crates/vim/src/helix.rs`<br>M `crates/vim/src/test.rs` | `impl Editor {`<br>`mod test {`<br>`async fn test_visual_rename_uses_visible_cursor_position(cx: &mut gpui::TestAppC` | `crates/vim/src/test.rs` |
| ZUP-144 | `361f4285bd4281f0f89fa2a519bc5d447b5ee5bb` — v1.11.x preview for @Veykril | M `crates/zed/RELEASE_CHANNEL` | `-dev` | No dedicated upstream test/fixture file changed. |
| ZUP-145 | `9919727437503d713a4c2d5d4c28b87ce047d80c` — cli: Restore workspace by default when `cli_default_open_behavior="new_window"` (#60652) (cherry-pick to preview) (#60671) | M `crates/zed/src/zed/open_listener.rs` | `async fn open_workspaces(`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-146 | `938e26ac07d68aa4a87e616cb352eac7e5cc725a` — ci: Fix insufficient permissions for asset validation step (#60625) (cherry-pick to preview) (#60675) | M `.github/workflows/release.yml`<br>M `tooling/xtask/src/tasks/workflows/release.rs` | `jobs:`<br>`fn validate_release_assets(deps: &[&NamedJob]) -> NamedJob {` | No dedicated upstream test/fixture file changed. |
| ZUP-147 | `99e33bad8930faff5bfc51bd41693e7d58510bf0` — open_ai: Add GPT 5.6 models (#60697) (cherry-pick to preview) (#60698) | M `crates/language_models/src/provider/open_ai.rs`<br>M `crates/open_ai/src/open_ai.rs` | `impl LanguageModel for OpenAiLanguageModel {`<br>`pub enum Model {`<br>`impl Model {` | No dedicated upstream test/fixture file changed. |
| ZUP-148 | `8a00e905d118362fc4a25b471d880c3108e50e75` — Bump to 1.11.1 for @bennetbo | M `Cargo.lock`<br>M `crates/zed/Cargo.toml` | `name = "zed"` | No dedicated upstream test/fixture file changed. |
| ZUP-149 | `fefc88b5834516cc168c471ddc9abb0171fca5d9` — agent_ui: Remove duplicate MCP Servers menu item (#60712) (cherry-pick to preview) (#60714) | M `crates/agent_ui/src/agent_panel.rs` | `impl AgentPanel {` | No dedicated upstream test/fixture file changed. |
| ZUP-150 | `da9588a0bc65d0bcce07f0fae83211432edea711` — agent: Add GPT 5.6 Sol/Terra for ChatGPT subscription (#60743) (cherry-pick to preview) (#60744) | M `crates/language_models/src/provider/openai_subscribed.rs` | `enum ChatGptModel {`<br>`impl ChatGptModel {`<br>`impl LanguageModel for OpenAiSubscribedLanguageModel {` | No dedicated upstream test/fixture file changed. |
| ZUP-151 | `3b24e487616bed6d60f27a791ec492bf465fbd7f` — Bump to 1.11.2 for @bennetbo | M `Cargo.lock`<br>M `crates/zed/Cargo.toml` | `name = "zed"` | No dedicated upstream test/fixture file changed. |
| ZUP-152 | `0f168fc8ca191f7fea315be998dfd4cfc6c60b56` — agent: Fix cmd-f not working when buffer is open (#60750) (cherry-pick to preview) (#60754) | M `assets/keymaps/default-linux.json`<br>M `assets/keymaps/default-macos.json`<br>M `assets/keymaps/default-windows.json`<br>M `crates/agent_ui/src/agent_panel.rs` | `-  {`<br>`+      "ctrl-f": "agent::ToggleSearch",`<br>`+    "use_key_equivalents": true,`<br>`+      "cmd-f": "agent::ToggleSearch",`<br>`impl Render for AgentPanel {` | No dedicated upstream test/fixture file changed. |
| ZUP-153 | `1f49eedcff39a96d04b8626209fee7cade52e152` — node_runtime: Fix npm v12 output deserialization (#60798) (cherry-pick to preview) (#60869) | M `crates/node_runtime/src/node_runtime.rs` | `impl NodeRuntime {`<br>`pub struct NpmInfo {`<br>`mod tests {` | No dedicated upstream test/fixture file changed. |
| ZUP-154 | `2b71e458379590339d96baa7ea78548f9243c899` — Bump to 1.11.3 for @MrSubidubi | M `Cargo.lock`<br>M `crates/zed/Cargo.toml` | `name = "zed"` | No dedicated upstream test/fixture file changed. |
| ZUP-155 | `08b45cd39af7fa7a3046ab9d29f6e028f381ebe9` — Fix "Task polled after completion" panic (#60693) (cherry-pick to preview) (#60886) | M `crates/gpui_linux/src/linux/dispatcher.rs`<br>M `crates/gpui_windows/src/dispatcher.rs` | `impl PlatformDispatcher for LinuxDispatcher {`<br>`use std::{`<br>`use gpui_util::ResultExt;`<br>`use windows::{`<br>`impl WindowsDispatcher {`<br>`impl PlatformDispatcher for WindowsDispatcher {` | No dedicated upstream test/fixture file changed. |
| ZUP-156 | `ab70b2ca973ba67e15a42d7039beb82ac87a21ee` — macos: Fix window move controls are disabled (#60620) (cherry-pick to preview) (#60966) | M `crates/gpui/Cargo.toml`<br>A `crates/gpui/examples/window_movable.rs`<br>M `crates/gpui/src/platform.rs`<br>M `crates/gpui/src/window.rs`<br>M `crates/gpui_macos/src/window.rs`<br>M `crates/zed/src/zed.rs` | `path = "examples/on_window_close_quit.rs"`<br>`+#![cfg_attr(target_family = "wasm", no_main)]`<br>`pub struct WindowOptions {`<br>`pub struct WindowParams {`<br>`impl Default for WindowOptions {`<br>`impl Window {`<br>`unsafe fn build_classes() {`<br>`struct MacWindowState {` | No dedicated upstream test/fixture file changed. |
| ZUP-157 | `0b231d618095d9db23897571808393173ff5638b` — languages: Pin TypeScript to 6.x for typescript-language-server (#60970) (cherry-pick to preview) (#60980) | M `crates/languages/src/typescript.rs`<br>M `crates/node_runtime/src/node_runtime.rs` | `use project::{Fs, lsp_store::language_server_settings};`<br>`fn replace_test_name_parameters(test_name: &str) -> String {`<br>`impl TypeScriptLspAdapter {`<br>`impl LspInstaller for TypeScriptLspAdapter {`<br>`use log::Level;`<br>`impl NodeRuntime {`<br>`fn select_npm_package_version(`<br>`fn is_allowed_npm_version_before(` | No dedicated upstream test/fixture file changed. |
| ZUP-158 | `884e31a976374f98c9249cbf63361491cab50ac9` — settings_content: Fix globally excluded language servers starting (#60984) (cherry-pick to preview) (#60987) | M `crates/settings_content/src/language.rs` | `impl merge_from::MergeFrom for AllLanguageSettingsContent {`<br>`mod test {` | No dedicated upstream test/fixture file changed. |
| ZUP-159 | `b5ccddc399da7078d23ef38a7a265e358ffee584` — fs: Update trash crate version (#60899) (cherry-pick to preview) (#61033) | M `Cargo.lock`<br>M `crates/fs/Cargo.toml` | `version = "5.2.5"`<br>`notify = "9.0.0-rc.4"` | No dedicated upstream test/fixture file changed. |
| ZUP-160 | `952d712dac48a4af2c54fb22c82d82a9d69b72d4` — v1.11.x stable for @JosephTLyons | M `crates/zed/RELEASE_CHANNEL` | `-preview` | No dedicated upstream test/fixture file changed. |

## Independently reviewable split within ZUP-091

Review of ZUP-091 found two unrelated changes in one upstream commit. The commit
ledger remains the commit-accounting authority, while the port matrix assigns
separate stable change IDs:

- `ZUP-091-LICENSE`: add `tooling/lints/LICENSE-APACHE`, dependent on the
  ZUP-067 lint workspace.
- `ZUP-091-ORCHESTRATE`: make the generated and generator-owned `grep` pipeline
  tolerate no matches under `pipefail` in `.github/workflows/run_tests.yml` and
  `tooling/xtask/src/tasks/workflows/run_tests.rs`.

Thus 160 commits produce 161 independently reviewable port decisions.

## Endpoint changed-path ledger

This ledger accounts for every added, modified, deleted, and renamed endpoint path. Current relation is content-addressed against the supplied local Sim filesystem and the verified upstream trees; it is merge-strategy evidence, not a port decision.

| Status | Endpoint path | Category | Right-range change IDs | Current relation |
| --- | --- | --- | --- | --- |
| A | `.agents/skills/lint-creator/SKILL.md` | production | ZUP-067 | absent locally |
| M | `.github/CODEOWNERS.hold` | CI/release | ZUP-043 | Sim-diverged |
| M | `.github/workflows/after_release.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/autofix_pr.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/bump_collab_staging.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/bump_patch_version.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/bump_zed_version.yml` | CI/release | ZUP-090 | absent locally |
| M | `.github/workflows/cherry_pick.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/comment_on_potential_duplicate_issues.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/community_close_stale_issues.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/community_update_all_top_ranking_issues.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/community_update_weekly_top_ranking_issues.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/compliance_check.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/congrats.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/danger.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/deploy_collab.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/deploy_docs.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/deploy_nightly_docs.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/docs_suggestions.yml` | CI/release | ZUP-090 | equals v1.10.2 |
| M | `.github/workflows/extension_auto_bump.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/extension_bump.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/extension_tests.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/extension_workflow_rollout.yml` | CI/release | ZUP-082, ZUP-090 | Sim-diverged |
| M | `.github/workflows/good_first_issue_notifier.yml` | CI/release | ZUP-090 | Sim-diverged |
| A | `.github/workflows/guild_assignment_status.yml` | CI/release | ZUP-016 | absent locally |
| A | `.github/workflows/guild_new_pr_notify.yml` | CI/release | ZUP-016 | absent locally |
| A | `.github/workflows/guild_stale_assignments.yml` | CI/release | ZUP-016 | absent locally |
| A | `.github/workflows/guild_weekly_shipped.yml` | CI/release | ZUP-016 | absent locally |
| M | `.github/workflows/hotfix-review-monitor.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/nix_build.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/pr_issue_labeler.yml` | CI/release | ZUP-016 | Sim-diverged |
| M | `.github/workflows/publish_extension_cli.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/release.yml` | CI/release | ZUP-090, ZUP-146 | Sim-diverged |
| M | `.github/workflows/release_nightly.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/run_bundling.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/run_tests.yml` | CI/release | ZUP-090, ZUP-091, ZUP-096 | Sim-diverged |
| M | `.github/workflows/slack_notify_community_automation_failure.yml` | CI/release | ZUP-016, ZUP-090 | Sim-diverged |
| M | `.github/workflows/slack_notify_first_responders.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/slack_notify_label_created.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/stale-pr-reminder.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `.github/workflows/update_duplicate_magnets.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `Cargo.lock` | manifest/dependency | ZUP-002, ZUP-003, ZUP-011, ZUP-015, ZUP-033, ZUP-044, ZUP-047, ZUP-050, ZUP-061, ZUP-099, ZUP-102, ZUP-113, ZUP-117, ZUP-128, ZUP-130, ZUP-148, ZUP-151, ZUP-154, ZUP-159 | Sim-diverged |
| M | `Cargo.toml` | manifest/dependency | ZUP-015, ZUP-061, ZUP-067, ZUP-099, ZUP-117, ZUP-128 | Sim-diverged |
| M | `assets/icons/folder_share.svg` | asset/settings | ZUP-004 | equals v1.10.2 |
| M | `assets/icons/folder_shared.svg` | asset/settings | ZUP-004 | equals v1.10.2 |
| M | `assets/icons/square_dot.svg` | asset/settings | ZUP-088 | equals v1.10.2 |
| M | `assets/icons/square_minus.svg` | asset/settings | ZUP-088 | equals v1.10.2 |
| M | `assets/icons/square_plus.svg` | asset/settings | ZUP-088 | equals v1.10.2 |
| A | `assets/icons/user_arrow_up.svg` | asset/settings | ZUP-004 | absent locally |
| M | `assets/keymaps/default-linux.json` | keymap/action | ZUP-035, ZUP-152 | Sim-diverged |
| M | `assets/keymaps/default-macos.json` | keymap/action | ZUP-035, ZUP-152 | Sim-diverged |
| M | `assets/keymaps/default-windows.json` | keymap/action | ZUP-035, ZUP-152 | Sim-diverged |
| M | `assets/keymaps/linux/jetbrains.json` | keymap/action | ZUP-063 | Sim-diverged |
| M | `assets/keymaps/macos/jetbrains.json` | keymap/action | ZUP-063 | Sim-diverged |
| M | `assets/keymaps/specific-overrides-macos.json` | keymap/action | ZUP-029 | equals v1.10.2 |
| M | `assets/keymaps/specific-overrides.json` | keymap/action | ZUP-029 | equals v1.10.2 |
| D | `assets/keymaps/storybook.json` | keymap/action | ZUP-043 | equals v1.10.2 |
| M | `assets/settings/default.json` | asset/settings | ZUP-009, ZUP-021, ZUP-140 | Sim-diverged |
| M | `crates/acp_thread/src/acp_thread.rs` | production | ZUP-048, ZUP-142 | Sim-diverged |
| M | `crates/acp_thread/src/connection.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/acp_thread/src/diff.rs` | production | ZUP-137 | equals v1.10.2 |
| M | `crates/acp_thread/src/mention.rs` | production | ZUP-071 | Sim-diverged |
| M | `crates/agent/src/agent.rs` | production | ZUP-078 | Sim-diverged |
| M | `crates/agent/src/db.rs` | production | ZUP-134 | Sim-diverged |
| M | `crates/agent/src/thread.rs` | production | ZUP-078 | Sim-diverged |
| M | `crates/agent/src/tools/find_path_tool.rs` | production | ZUP-071 | equals v1.10.2 |
| M | `crates/agent/src/tools/grep_tool.rs` | production | ZUP-071 | Sim-diverged |
| M | `crates/agent_servers/src/acp.rs` | production | ZUP-080 | Sim-diverged |
| M | `crates/agent_ui/Cargo.toml` | manifest/dependency | ZUP-003 | Sim-diverged |
| M | `crates/agent_ui/src/agent_diff.rs` | production | ZUP-088, ZUP-098 | Sim-diverged |
| M | `crates/agent_ui/src/agent_panel.rs` | production | ZUP-003, ZUP-004, ZUP-035, ZUP-083, ZUP-149, ZUP-152 | Sim-diverged |
| M | `crates/agent_ui/src/agent_registry_ui.rs` | production | ZUP-083 | Sim-diverged |
| M | `crates/agent_ui/src/agent_ui.rs` | production | ZUP-071, ZUP-078 | Sim-diverged |
| M | `crates/agent_ui/src/completion_provider.rs` | production | ZUP-004 | Sim-diverged |
| M | `crates/agent_ui/src/config_options.rs` | production | ZUP-080 | Sim-diverged |
| M | `crates/agent_ui/src/conversation_view.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/agent_ui/src/conversation_view/elicitation.rs` | production | ZUP-117 | Sim-diverged |
| M | `crates/agent_ui/src/conversation_view/thread_view.rs` | production | ZUP-004, ZUP-030, ZUP-069, ZUP-071, ZUP-078, ZUP-088 | Sim-diverged |
| M | `crates/agent_ui/src/diagnostics.rs` | production | ZUP-137 | equals v1.10.2 |
| M | `crates/agent_ui/src/entry_view_state.rs` | production | ZUP-098 | equals v1.10.2 |
| M | `crates/agent_ui/src/message_editor.rs` | production | ZUP-004 | Sim-diverged |
| M | `crates/agent_ui/src/ui/mention_crease.rs` | production | ZUP-071 | Sim-diverged |
| M | `crates/auto_update/src/auto_update.rs` | production | ZUP-036, ZUP-038, ZUP-042, ZUP-068, ZUP-092, ZUP-112 | Sim-diverged |
| M | `crates/bedrock/src/models.rs` | production | ZUP-051, ZUP-055, ZUP-070, ZUP-128 | equals v1.10.2 |
| M | `crates/buffer_diff/src/buffer_diff.rs` | production | ZUP-098 | Sim-diverged |
| M | `crates/client/src/test.rs` | test/fixture | ZUP-094 | Sim-diverged |
| M | `crates/client/src/zed_urls.rs` | production | ZUP-050 | absent locally |
| M | `crates/cloud_api_client/src/cloud_api_client.rs` | production | ZUP-129 | Sim-diverged |
| M | `crates/cloud_api_types/src/cloud_api_types.rs` | production | ZUP-022, ZUP-094 | Sim-diverged |
| M | `crates/collab/src/auth.rs` | production | ZUP-094 | Sim-diverged |
| M | `crates/collab/src/db.rs` | production | ZUP-034 | equals v1.10.2 |
| M | `crates/collab/src/rpc.rs` | production | ZUP-034 | Sim-diverged |
| M | `crates/collab/tests/integration/channel_tests.rs` | test/fixture | ZUP-045 | Sim-diverged |
| M | `crates/collab/tests/integration/test_server.rs` | test/fixture | ZUP-045 | Sim-diverged |
| M | `crates/command_palette/src/command_palette.rs` | production | ZUP-028 | Sim-diverged |
| M | `crates/context_server/src/client.rs` | production | ZUP-058 | equals v1.10.2 |
| M | `crates/context_server/src/context_server.rs` | production | ZUP-058 | Sim-diverged |
| M | `crates/context_server/src/protocol.rs` | production | ZUP-058 | Sim-diverged |
| M | `crates/context_server/src/transport.rs` | production | ZUP-058 | equals v1.10.2 |
| M | `crates/context_server/src/transport/http.rs` | production | ZUP-058 | equals v1.10.2 |
| M | `crates/copilot_ui/Cargo.toml` | manifest/dependency | ZUP-113 | equals v1.10.2 |
| M | `crates/copilot_ui/src/sign_in.rs` | production | ZUP-113 | Sim-diverged |
| M | `crates/dev_container/src/devcontainer_manifest.rs` | production | ZUP-060 | Sim-diverged |
| M | `crates/edit_prediction_cli/src/filter_languages.rs` | production | ZUP-139 | equals v1.10.2 |
| M | `crates/edit_prediction_ui/src/rate_prediction_modal.rs` | production | ZUP-137 | Sim-diverged |
| M | `crates/editor/Cargo.toml` | manifest/dependency | ZUP-130 | Sim-diverged |
| M | `crates/editor/src/bracket_colorization.rs` | production | ZUP-005 | equals v1.10.2 |
| M | `crates/editor/src/clipboard.rs` | production | ZUP-019 | Sim-diverged |
| M | `crates/editor/src/code_context_menus.rs` | production | ZUP-093 | equals v1.10.2 |
| M | `crates/editor/src/completions.rs` | production | ZUP-025 | Sim-diverged |
| M | `crates/editor/src/config.rs` | production | ZUP-098 | equals v1.10.2 |
| M | `crates/editor/src/edit_prediction.rs` | production | ZUP-137 | Sim-diverged |
| M | `crates/editor/src/editor.rs` | production | ZUP-005, ZUP-018, ZUP-026, ZUP-031, ZUP-079, ZUP-088, ZUP-098, ZUP-137, ZUP-143 | Sim-diverged |
| M | `crates/editor/src/editor_tests.rs` | test/fixture | ZUP-019, ZUP-025, ZUP-026, ZUP-031, ZUP-101 | Sim-diverged |
| M | `crates/editor/src/element.rs` | production | ZUP-018, ZUP-088, ZUP-098 | Sim-diverged |
| M | `crates/editor/src/element/header.rs` | production | ZUP-057, ZUP-088, ZUP-137 | Sim-diverged |
| M | `crates/editor/src/git.rs` | production | ZUP-079, ZUP-098 | equals v1.10.2 |
| M | `crates/editor/src/git/blame.rs` | production | ZUP-079 | equals v1.10.2 |
| M | `crates/editor/src/hover_popover.rs` | production | ZUP-130 | Sim-diverged |
| M | `crates/editor/src/input.rs` | production | ZUP-133 | equals v1.10.2 |
| M | `crates/editor/src/items.rs` | production | ZUP-137 | Sim-diverged |
| M | `crates/editor/src/split.rs` | production | ZUP-088, ZUP-098 | Sim-diverged |
| M | `crates/extension_host/src/extension_store_test.rs` | test/fixture | ZUP-127 | Sim-diverged |
| M | `crates/file_finder/src/file_finder.rs` | production | ZUP-133 | Sim-diverged |
| M | `crates/fs/Cargo.toml` | manifest/dependency | ZUP-159 | Sim-diverged |
| M | `crates/fs/src/fake_git_repo.rs` | production | ZUP-122 | equals v1.10.2 |
| M | `crates/fs/src/fs.rs` | production | ZUP-024, ZUP-033, ZUP-124 | Sim-diverged |
| M | `crates/fs/src/fs_watcher.rs` | production | ZUP-124 | Sim-diverged |
| M | `crates/git/src/commit.rs` | production | ZUP-074 | equals v1.10.2 |
| M | `crates/git/src/repository.rs` | production | ZUP-074, ZUP-122 | Sim-diverged |
| M | `crates/git_ui/Cargo.toml` | manifest/dependency | ZUP-050 | Sim-diverged |
| A | `crates/git_ui/src/branch_diff.rs` | production | ZUP-098 | absent locally |
| M | `crates/git_ui/src/commit_modal.rs` | production | ZUP-088 | Sim-diverged |
| M | `crates/git_ui/src/commit_view.rs` | production | ZUP-098 | Sim-diverged |
| M | `crates/git_ui/src/conflict_view.rs` | production | ZUP-098 | Sim-diverged |
| A | `crates/git_ui/src/diff_multibuffer.rs` | production | ZUP-098 | absent locally |
| M | `crates/git_ui/src/file_diff_view.rs` | production | ZUP-098, ZUP-137 | equals v1.10.2 |
| M | `crates/git_ui/src/git_graph.rs` | production | ZUP-072, ZUP-103, ZUP-119 | Sim-diverged |
| M | `crates/git_ui/src/git_panel.rs` | production | ZUP-050, ZUP-052, ZUP-059, ZUP-066, ZUP-076, ZUP-088, ZUP-098, ZUP-119, ZUP-121, ZUP-123, ZUP-126 | Sim-diverged |
| M | `crates/git_ui/src/git_ui.rs` | production | ZUP-088, ZUP-098 | Sim-diverged |
| M | `crates/git_ui/src/multi_diff_view.rs` | production | ZUP-098, ZUP-137 | equals v1.10.2 |
| M | `crates/git_ui/src/project_diff.rs` | production | ZUP-088, ZUP-098, ZUP-119 | Sim-diverged |
| M | `crates/git_ui/src/remote_output.rs` | production | ZUP-126 | equals v1.10.2 |
| M | `crates/git_ui/src/solo_diff_view.rs` | production | ZUP-088 | equals v1.10.2 |
| A | `crates/git_ui/src/staged_diff.rs` | production | ZUP-098 | absent locally |
| M | `crates/git_ui/src/text_diff_view.rs` | production | ZUP-098, ZUP-137 | Sim-diverged |
| A | `crates/git_ui/src/unstaged_diff.rs` | production | ZUP-098 | absent locally |
| M | `crates/gpui/Cargo.toml` | manifest/dependency | ZUP-156 | Sim-diverged |
| A | `crates/gpui/examples/window_movable.rs` | platform/GPUI | ZUP-156 | absent locally |
| M | `crates/gpui/src/elements/animation.rs` | platform/GPUI | ZUP-105 | equals v1.10.2 |
| M | `crates/gpui/src/elements/div.rs` | platform/GPUI | ZUP-114 | Sim-diverged |
| M | `crates/gpui/src/platform.rs` | platform/GPUI | ZUP-104, ZUP-156 | Sim-diverged |
| M | `crates/gpui/src/svg_renderer.rs` | platform/GPUI | ZUP-116 | equals v1.10.2 |
| M | `crates/gpui/src/window.rs` | platform/GPUI | ZUP-104, ZUP-115, ZUP-156 | Sim-diverged |
| M | `crates/gpui_linux/src/linux/dispatcher.rs` | platform/GPUI | ZUP-155 | equals v1.10.2 |
| M | `crates/gpui_linux/src/linux/headless.rs` | platform/GPUI | ZUP-053 | equals v1.10.2 |
| M | `crates/gpui_linux/src/linux/headless/client.rs` | platform/GPUI | ZUP-053 | equals v1.10.2 |
| A | `crates/gpui_linux/src/linux/headless/window.rs` | platform/GPUI | ZUP-053 | absent locally |
| M | `crates/gpui_linux/src/linux/wayland/client.rs` | platform/GPUI | ZUP-065 | Sim-diverged |
| M | `crates/gpui_linux/src/linux/wayland/window.rs` | platform/GPUI | ZUP-104 | equals v1.10.2 |
| M | `crates/gpui_linux/src/linux/x11/window.rs` | platform/GPUI | ZUP-115 | Sim-diverged |
| M | `crates/gpui_macos/src/text_system.rs` | platform/GPUI | ZUP-106 | equals v1.10.2 |
| M | `crates/gpui_macos/src/window.rs` | platform/GPUI | ZUP-108, ZUP-156 | Sim-diverged |
| M | `crates/gpui_wgpu/src/wgpu_renderer.rs` | platform/GPUI | ZUP-125 | Sim-diverged |
| M | `crates/gpui_windows/src/dispatcher.rs` | platform/GPUI | ZUP-155 | equals v1.10.2 |
| M | `crates/gpui_windows/src/events.rs` | platform/GPUI | ZUP-107 | equals v1.10.2 |
| M | `crates/gpui_windows/src/platform.rs` | platform/GPUI | ZUP-064 | Sim-diverged |
| M | `crates/gpui_windows/src/window.rs` | platform/GPUI | ZUP-107 | Sim-diverged |
| M | `crates/grammars/src/bash/config.toml` | production | ZUP-139 | equals v1.10.2 |
| M | `crates/grammars/src/go/outline.scm` | production | ZUP-136 | equals v1.10.2 |
| M | `crates/grammars/src/javascript/outline.scm` | production | ZUP-007 | equals v1.10.2 |
| M | `crates/grammars/src/javascript/runnables.scm` | production | ZUP-007 | equals v1.10.2 |
| M | `crates/grammars/src/tsx/outline.scm` | production | ZUP-007 | equals v1.10.2 |
| M | `crates/grammars/src/tsx/runnables.scm` | production | ZUP-007 | equals v1.10.2 |
| M | `crates/grammars/src/typescript/outline.scm` | production | ZUP-007 | equals v1.10.2 |
| M | `crates/grammars/src/typescript/runnables.scm` | production | ZUP-007 | equals v1.10.2 |
| M | `crates/icons/src/icons.rs` | production | ZUP-004 | Sim-diverged |
| M | `crates/keymap_editor/src/keymap_editor.rs` | production | ZUP-037, ZUP-134 | Sim-diverged |
| M | `crates/language/src/buffer.rs` | production | ZUP-006, ZUP-100 | Sim-diverged |
| M | `crates/language/src/buffer_tests.rs` | test/fixture | ZUP-006, ZUP-100 | Sim-diverged |
| M | `crates/language/src/language_settings.rs` | production | ZUP-041 | Sim-diverged |
| M | `crates/language_extension/src/extension_lsp_adapter.rs` | production | ZUP-127 | Sim-diverged |
| M | `crates/language_extension/src/language_extension.rs` | production | ZUP-127 | equals v1.10.2 |
| M | `crates/language_model/src/fake_provider.rs` | production | ZUP-030 | equals v1.10.2 |
| M | `crates/language_model/src/language_model.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_model/src/registry.rs` | production | ZUP-078 | Sim-diverged |
| M | `crates/language_model_core/Cargo.toml` | manifest/dependency | ZUP-102 | equals v1.10.2 |
| M | `crates/language_model_core/src/tool_schema.rs` | production | ZUP-102 | equals v1.10.2 |
| M | `crates/language_models/Cargo.toml` | manifest/dependency | ZUP-128 | equals v1.10.2 |
| M | `crates/language_models/src/language_models.rs` | production | ZUP-078 | Sim-diverged |
| M | `crates/language_models/src/provider/anthropic.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/anthropic_compatible.rs` | production | ZUP-030 | equals v1.10.2 |
| M | `crates/language_models/src/provider/bedrock.rs` | production | ZUP-030, ZUP-051, ZUP-056, ZUP-128 | Sim-diverged |
| M | `crates/language_models/src/provider/cloud.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/copilot_chat.rs` | production | ZUP-030 | equals v1.10.2 |
| M | `crates/language_models/src/provider/deepseek.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/google.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/llama_cpp.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/lmstudio.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/mistral.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/ollama.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/open_ai.rs` | production | ZUP-030, ZUP-147 | Sim-diverged |
| M | `crates/language_models/src/provider/open_ai_compatible.rs` | production | ZUP-030 | equals v1.10.2 |
| M | `crates/language_models/src/provider/open_router.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/openai_subscribed.rs` | production | ZUP-030, ZUP-064, ZUP-150 | Sim-diverged |
| M | `crates/language_models/src/provider/opencode.rs` | production | ZUP-030, ZUP-111 | Sim-diverged |
| M | `crates/language_models/src/provider/vercel_ai_gateway.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/provider/x_ai.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/language_models/src/settings.rs` | production | ZUP-128 | Sim-diverged |
| M | `crates/language_tools/src/lsp_button.rs` | production | ZUP-020 | Sim-diverged |
| M | `crates/languages/src/go.rs` | production | ZUP-136 | Sim-diverged |
| M | `crates/languages/src/typescript.rs` | production | ZUP-007, ZUP-157 | Sim-diverged |
| M | `crates/livekit_api/Cargo.toml` | manifest/dependency | ZUP-045 | equals v1.10.2 |
| M | `crates/livekit_api/src/livekit_api.rs` | production | ZUP-045 | equals v1.10.2 |
| M | `crates/livekit_api/src/token.rs` | production | ZUP-045 | equals v1.10.2 |
| M | `crates/livekit_client/Cargo.toml` | manifest/dependency | ZUP-045 | equals v1.10.2 |
| M | `crates/livekit_client/src/test.rs` | test/fixture | ZUP-045 | equals v1.10.2 |
| M | `crates/markdown/Cargo.toml` | manifest/dependency | ZUP-011 | equals v1.10.2 |
| M | `crates/markdown/src/html/html_rendering.rs` | production | ZUP-133 | equals v1.10.2 |
| M | `crates/markdown/src/markdown.rs` | production | ZUP-009, ZUP-011 | Sim-diverged |
| M | `crates/multi_buffer/src/multi_buffer.rs` | production | ZUP-098, ZUP-137 | Sim-diverged |
| M | `crates/node_runtime/src/node_runtime.rs` | production | ZUP-153, ZUP-157 | Sim-diverged |
| M | `crates/onboarding/src/basics_page.rs` | production | ZUP-083 | Sim-diverged |
| M | `crates/open_ai/src/completion.rs` | production | ZUP-128 | Sim-diverged |
| M | `crates/open_ai/src/responses.rs` | production | ZUP-128 | equals v1.10.2 |
| M | `crates/opencode/src/opencode.rs` | production | ZUP-111 | equals v1.10.2 |
| M | `crates/outline_panel/src/outline_panel.rs` | production | ZUP-119, ZUP-133 | Sim-diverged |
| M | `crates/picker/src/footer.rs` | production | ZUP-088 | equals v1.10.2 |
| M | `crates/picker/src/picker.rs` | production | ZUP-086 | Sim-diverged |
| M | `crates/picker/src/preview.rs` | production | ZUP-047 | equals v1.10.2 |
| M | `crates/picker_preview/Cargo.toml` | manifest/dependency | ZUP-047 | equals v1.10.2 |
| M | `crates/picker_preview/src/picker_preview.rs` | production | ZUP-047 | equals v1.10.2 |
| M | `crates/project/src/context_server_store.rs` | production | ZUP-058 | Sim-diverged |
| M | `crates/project/src/git_store.rs` | production | ZUP-012, ZUP-066, ZUP-098, ZUP-122 | Sim-diverged |
| R076 | `crates/project/src/git_store/branch_diff.rs` → `crates/project/src/git_store/diff_buffer_list.rs` | production | ZUP-098 | absent locally |
| M | `crates/project/src/lsp_store.rs` | production | ZUP-006, ZUP-101, ZUP-118, ZUP-132, ZUP-135 | Sim-diverged |
| M | `crates/project/src/project.rs` | production | ZUP-017, ZUP-034, ZUP-098 | Sim-diverged |
| M | `crates/project/src/project_search.rs` | production | ZUP-017, ZUP-087 | equals v1.10.2 |
| M | `crates/project/src/search.rs` | production | ZUP-017 | equals v1.10.2 |
| M | `crates/project/src/worktree_store.rs` | production | ZUP-034 | Sim-diverged |
| M | `crates/project/tests/integration/context_server_store.rs` | test/fixture | ZUP-058 | equals v1.10.2 |
| M | `crates/project/tests/integration/project_tests.rs` | test/fixture | ZUP-034, ZUP-098, ZUP-135 | Sim-diverged |
| M | `crates/project_panel/Cargo.toml` | manifest/dependency | ZUP-033 | Sim-diverged |
| M | `crates/project_panel/src/project_panel.rs` | production | ZUP-013, ZUP-033, ZUP-114, ZUP-119, ZUP-141 | Sim-diverged |
| M | `crates/project_panel/src/project_panel_tests.rs` | test/fixture | ZUP-013 | Sim-diverged |
| M | `crates/project_panel/src/tests/undo.rs` | test/fixture | ZUP-033 | equals v1.10.2 |
| M | `crates/project_symbols/Cargo.toml` | manifest/dependency | ZUP-047 | equals v1.10.2 |
| M | `crates/project_symbols/src/project_symbols.rs` | production | ZUP-047 | Sim-diverged |
| M | `crates/proto/proto/call.proto` | protocol/schema | ZUP-034 | Sim-diverged |
| M | `crates/proto/proto/git.proto` | protocol/schema | ZUP-098 | Sim-diverged |
| M | `crates/proto/proto/worktree.proto` | protocol/schema | ZUP-034 | Sim-diverged |
| M | `crates/proto/src/proto.rs` | protocol/schema | ZUP-034 | Sim-diverged |
| M | `crates/remote/src/transport/ssh.rs` | production | ZUP-134 | Sim-diverged |
| M | `crates/remote/src/transport/wsl.rs` | production | ZUP-134 | Sim-diverged |
| M | `crates/remote_server/src/headless_project.rs` | production | ZUP-017, ZUP-034, ZUP-127 | Sim-diverged |
| M | `crates/remote_server/src/remote_editing_tests.rs` | test/fixture | ZUP-034 | Sim-diverged |
| M | `crates/reqwest_client/src/reqwest_client.rs` | production | ZUP-036, ZUP-038, ZUP-039 | equals v1.10.2 |
| M | `crates/search/src/buffer_search.rs` | production | ZUP-088, ZUP-098 | Sim-diverged |
| M | `crates/search/src/text_finder.rs` | production | ZUP-029, ZUP-085, ZUP-087 | Sim-diverged |
| M | `crates/search/src/text_finder/delegate.rs` | production | ZUP-029, ZUP-087 | equals v1.10.2 |
| M | `crates/search/src/text_finder/render.rs` | production | ZUP-029 | equals v1.10.2 |
| M | `crates/session/src/session.rs` | production | ZUP-118 | equals v1.10.2 |
| M | `crates/settings/src/vscode_import.rs` | production | ZUP-021, ZUP-140 | Sim-diverged |
| M | `crates/settings_content/src/language.rs` | production | ZUP-041, ZUP-158 | Sim-diverged |
| M | `crates/settings_content/src/language_model.rs` | production | ZUP-111, ZUP-128 | Sim-diverged |
| M | `crates/settings_content/src/settings_content.rs` | production | ZUP-021 | Sim-diverged |
| M | `crates/settings_content/src/terminal.rs` | production | ZUP-140 | equals v1.10.2 |
| M | `crates/settings_content/src/theme.rs` | production | ZUP-009 | Sim-diverged |
| M | `crates/settings_ui/src/page_data.rs` | production | ZUP-021, ZUP-140 | Sim-diverged |
| M | `crates/settings_ui/src/pages/edit_prediction_provider_setup.rs` | production | ZUP-110 | Sim-diverged |
| M | `crates/settings_ui/src/pages/llm_providers_page.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/settings_ui/src/settings_ui.rs` | production | ZUP-030 | Sim-diverged |
| M | `crates/terminal/src/alacritty.rs` | production | ZUP-048 | Sim-diverged |
| M | `crates/terminal/src/terminal.rs` | production | ZUP-048, ZUP-140 | Sim-diverged |
| M | `crates/terminal/src/terminal_settings.rs` | production | ZUP-140 | equals v1.10.2 |
| M | `crates/terminal_view/Cargo.toml` | manifest/dependency | ZUP-003 | Sim-diverged |
| M | `crates/terminal_view/src/terminal_element.rs` | production | ZUP-065 | Sim-diverged |
| M | `crates/terminal_view/src/terminal_panel.rs` | production | ZUP-049 | Sim-diverged |
| M | `crates/terminal_view/src/terminal_view.rs` | production | ZUP-003, ZUP-065 | Sim-diverged |
| A | `crates/theme/src/color_space.rs` | production | ZUP-005 | absent locally |
| M | `crates/theme/src/theme.rs` | production | ZUP-005 | Sim-diverged |
| M | `crates/theme_settings/src/settings.rs` | production | ZUP-009 | Sim-diverged |
| M | `crates/title_bar/src/update_version.rs` | production | ZUP-042, ZUP-068, ZUP-092 | equals v1.10.2 |
| M | `crates/ui/src/components/button/icon_button.rs` | production | ZUP-050 | Sim-diverged |
| M | `crates/ui/src/components/button/split_button.rs` | production | ZUP-088 | equals v1.10.2 |
| M | `crates/ui/src/components/collab/update_button.rs` | production | ZUP-042 | Sim-diverged |
| M | `crates/ui/src/components/context_menu.rs` | production | ZUP-103 | equals v1.10.2 |
| M | `crates/ui/src/components/data_table.rs` | production | ZUP-103, ZUP-119 | equals v1.10.2 |
| M | `crates/ui/src/components/data_table/table_row.rs` | production | ZUP-103 | equals v1.10.2 |
| M | `crates/ui/src/components/data_table/tests.rs` | test/fixture | ZUP-103 | equals v1.10.2 |
| M | `crates/ui/src/components/divider.rs` | production | ZUP-088 | equals v1.10.2 |
| M | `crates/ui/src/components/indent_guides.rs` | production | ZUP-119 | equals v1.10.2 |
| M | `crates/ui/src/components/progress/circular_progress.rs` | production | ZUP-042 | equals v1.10.2 |
| M | `crates/ui/src/components/redistributable_columns.rs` | production | ZUP-103 | equals v1.10.2 |
| M | `crates/vim/src/command.rs` | production | ZUP-026 | Sim-diverged |
| M | `crates/vim/src/helix.rs` | production | ZUP-032, ZUP-143 | Sim-diverged |
| M | `crates/vim/src/normal.rs` | production | ZUP-026 | equals v1.10.2 |
| M | `crates/vim/src/test.rs` | test/fixture | ZUP-143 | Sim-diverged |
| M | `crates/vim/src/visual.rs` | production | ZUP-032 | Sim-diverged |
| M | `crates/workspace/src/multi_workspace.rs` | production | ZUP-027 | Sim-diverged |
| M | `crates/workspace/src/multi_workspace_tests.rs` | test/fixture | ZUP-027, ZUP-034 | equals v1.10.2 |
| M | `crates/workspace/src/pane.rs` | production | ZUP-013, ZUP-089, ZUP-109 | Sim-diverged |
| M | `crates/workspace/src/workspace.rs` | production | ZUP-131 | Sim-diverged |
| M | `crates/worktree/src/worktree.rs` | production | ZUP-034 | Sim-diverged |
| M | `crates/worktree/tests/integration/worktree_tests.rs` | test/fixture | ZUP-034 | Sim-diverged |
| M | `crates/zed/Cargo.toml` | manifest/dependency | ZUP-002, ZUP-148, ZUP-151, ZUP-154 | absent locally |
| M | `crates/zed/src/main.rs` | production | ZUP-073 | absent locally |
| M | `crates/zed/src/zed.rs` | production | ZUP-040, ZUP-075, ZUP-098, ZUP-156 | absent locally |
| M | `crates/zed/src/zed/app_menus.rs` | production | ZUP-123 | absent locally |
| M | `crates/zed/src/zed/open_listener.rs` | production | ZUP-023, ZUP-145 | absent locally |
| M | `crates/zed_actions/src/lib.rs` | production | ZUP-030, ZUP-040, ZUP-083, ZUP-098, ZUP-123 | absent locally |
| M | `crates/zeta_prompt/src/multi_region.rs` | production | ZUP-133 | equals v1.10.2 |
| M | `crates/zlog/src/filter.rs` | production | ZUP-001 | equals v1.10.2 |
| M | `docs/src/ai/agent-panel.md` | documentation | ZUP-054 | Sim-diverged |
| M | `docs/src/ai/edit-prediction.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/ai/mcp.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/ai/parallel-agents.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/ai/use-a-gateway.md` | documentation | ZUP-128 | Sim-diverged |
| M | `docs/src/ai/use-api-access.md` | documentation | ZUP-111 | Sim-diverged |
| M | `docs/src/authentication.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/command-palette.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/configuring-languages.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/debugger.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/development/glossary.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/development/windows.md` | documentation | ZUP-014 | Sim-diverged |
| M | `docs/src/diagnostics.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/environment.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/extensions/debugger-extensions.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/extensions/developing-extensions.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/finding-navigating.md` | documentation | ZUP-062 | Sim-diverged |
| M | `docs/src/git.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/ansible.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/cpp.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/elixir.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/gleam.md` | documentation | ZUP-081 | Sim-diverged |
| M | `docs/src/languages/go.md` | documentation | ZUP-084 | Sim-diverged |
| M | `docs/src/languages/java.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/javascript.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/json.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/kotlin.md` | documentation | ZUP-077 | Sim-diverged |
| M | `docs/src/languages/powershell.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/python.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/rust.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/languages/tailwindcss.md` | documentation | ZUP-081, ZUP-084 | Sim-diverged |
| M | `docs/src/languages/typescript.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/linux.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/performance.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/project-panel.md` | documentation | ZUP-138, ZUP-141 | Sim-diverged |
| M | `docs/src/reference/all-settings.md` | documentation | ZUP-138, ZUP-140 | Sim-diverged |
| M | `docs/src/remote-development.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/repl.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/terminal.md` | documentation | ZUP-140 | Sim-diverged |
| M | `docs/src/vim.md` | documentation | ZUP-138 | Sim-diverged |
| M | `docs/src/worktree-trust.md` | documentation | ZUP-138 | Sim-diverged |
| M | `extensions/workflows/run_tests.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `extensions/workflows/shared/bump_version.yml` | CI/release | ZUP-090 | Sim-diverged |
| M | `script/check-keymaps` | tooling/generated | ZUP-043 | equals v1.10.2 |
| M | `script/community-pr-track-mapping.json` | tooling/generated | ZUP-010 | Sim-diverged |
| M | `script/danger/package.json` | tooling/generated | ZUP-046 | equals v1.10.2 |
| M | `script/danger/pnpm-lock.yaml` | tooling/generated | ZUP-046 | equals v1.10.2 |
| M | `script/github-community-pr-board.py` | tooling/generated | ZUP-016 | Sim-diverged |
| A | `script/github-guild-board.py` | tooling/generated | ZUP-016 | absent locally |
| D | `script/storybook` | tooling/generated | ZUP-043 | equals v1.10.2 |
| M | `script/update_top_ranking_issues/uv.lock` | tooling/generated | ZUP-120 | equals v1.10.2 |
| A | `tooling/lints/.cargo/config.toml` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/.gitignore` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/Cargo.toml` | manifest/dependency | ZUP-067 | absent locally |
| A | `tooling/lints/LICENSE-APACHE` | tooling/generated | ZUP-091 | absent locally |
| A | `tooling/lints/README.md` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/rust-toolchain.toml` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/single-lint` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/src/blocking_io_on_foreground.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/src/entity_update_in_render.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/src/lib.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/src/notify_in_render.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/src/owned_string_into_shared.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/src/render_helpers.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/test_fixture/Cargo.toml` | manifest/dependency | ZUP-067 | absent locally |
| A | `tooling/lints/test_fixture/LICENSE-APACHE` | tooling/generated | ZUP-097 | absent locally |
| A | `tooling/lints/test_fixture/consumer/Cargo.toml` | manifest/dependency | ZUP-067 | absent locally |
| A | `tooling/lints/test_fixture/consumer/LICENSE-APACHE` | tooling/generated | ZUP-097 | absent locally |
| A | `tooling/lints/test_fixture/consumer/src/lib.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/test_fixture/gpui/Cargo.toml` | manifest/dependency | ZUP-067 | absent locally |
| A | `tooling/lints/test_fixture/gpui/LICENSE-APACHE` | tooling/generated | ZUP-097 | absent locally |
| A | `tooling/lints/test_fixture/gpui/src/lib.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/test_fixture/gpui_shared_string/Cargo.toml` | manifest/dependency | ZUP-067 | absent locally |
| A | `tooling/lints/test_fixture/gpui_shared_string/LICENSE-APACHE` | tooling/generated | ZUP-097 | absent locally |
| A | `tooling/lints/test_fixture/gpui_shared_string/src/lib.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/test_fixture/render_consumer/Cargo.toml` | manifest/dependency | ZUP-067 | absent locally |
| A | `tooling/lints/test_fixture/render_consumer/LICENSE-APACHE` | tooling/generated | ZUP-097 | absent locally |
| A | `tooling/lints/test_fixture/render_consumer/src/lib.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/ui/async_block_without_await.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/ui/async_block_without_await.stderr` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/ui/blocking_io_on_foreground.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/ui/blocking_io_on_foreground.stderr` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/ui/entity_update_in_render.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/ui/entity_update_in_render.stderr` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/ui/owned_string_into_shared.rs` | tooling/generated | ZUP-067 | absent locally |
| A | `tooling/lints/ui/owned_string_into_shared.stderr` | tooling/generated | ZUP-067 | absent locally |
| M | `tooling/xtask/src/tasks/workflow_checks.rs` | tooling/generated | ZUP-090 | equals v1.10.2 |
| A | `tooling/xtask/src/tasks/workflow_checks/check_permissions.rs` | tooling/generated | ZUP-090 | absent locally |
| M | `tooling/xtask/src/tasks/workflow_checks/check_run_patterns.rs` | tooling/generated | ZUP-090 | equals v1.10.2 |
| M | `tooling/xtask/src/tasks/workflows.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/after_release.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/autofix_pr.rs` | tooling/generated | ZUP-090 | equals v1.10.2 |
| M | `tooling/xtask/src/tasks/workflows/bump_patch_version.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/bump_zed_version.rs` | tooling/generated | ZUP-090 | absent locally |
| M | `tooling/xtask/src/tasks/workflows/cherry_pick.rs` | tooling/generated | ZUP-090 | equals v1.10.2 |
| M | `tooling/xtask/src/tasks/workflows/compliance_check.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/danger.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/deploy_collab.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/deploy_docs.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/extension_auto_bump.rs` | tooling/generated | ZUP-090 | equals v1.10.2 |
| M | `tooling/xtask/src/tasks/workflows/extension_bump.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/extension_tests.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/extension_workflow_rollout.rs` | tooling/generated | ZUP-082, ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/extensions/bump_version.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/extensions/run_tests.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/nix_build.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/publish_extension_cli.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/release.rs` | tooling/generated | ZUP-090, ZUP-146 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/release_nightly.rs` | tooling/generated | ZUP-090 | equals v1.10.2 |
| M | `tooling/xtask/src/tasks/workflows/run_bundling.rs` | tooling/generated | ZUP-090 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/run_tests.rs` | tooling/generated | ZUP-090, ZUP-091, ZUP-096 | Sim-diverged |
| M | `tooling/xtask/src/tasks/workflows/steps.rs` | tooling/generated | ZUP-090 | Sim-diverged |

## Net-unchanged right-range paths

These paths were touched by right-range commits but are identical between the two verified endpoints, or are represented by the rename ledger under a different endpoint name:

- `crates/cloud_llm_client/src/cloud_llm_client.rs` — ZUP-095
- `crates/codestral/src/codestral.rs` — ZUP-095
- `crates/edit_prediction/src/edit_prediction_tests.rs` — ZUP-095
- `crates/edit_prediction/src/prediction.rs` — ZUP-095
- `crates/edit_prediction/src/zed_edit_prediction_delegate.rs` — ZUP-095
- `crates/edit_prediction_types/src/edit_prediction_types.rs` — ZUP-095
- `crates/gpui/src/app.rs` — ZUP-036, ZUP-038
- `crates/open_ai/src/open_ai.rs` — ZUP-147
- `crates/sandbox/src/windows_wsl.rs` — ZUP-008
- `crates/settings/src/keymap_file.rs` — ZUP-037
- `crates/zed/RELEASE_CHANNEL` — ZUP-144, ZUP-160

## Completeness invariants

- Every one of the 160 right-exclusive commits has exactly one commit-ledger row;
  ZUP-091 is split into two independently reviewable port decisions.
- Every endpoint-changed path has exactly one endpoint ledger row.
- Every endpoint ledger row maps back to one or more right-range change IDs; no endpoint path is unexplained.
- Net-unchanged touched paths remain explicit and are not used to claim missing implementation.
- Release notes and documentation are supporting evidence only; executable decisions are recorded in port-matrix.md and verified through traceability.md and validation.md.
