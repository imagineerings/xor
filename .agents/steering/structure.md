# Project Structure

## Top-level

```
crates/          Main Sim Rust workspace — all production crates
extensions/      In-tree WASM extensions (glsl, html, proto, test-extension)
tooling/         Build tooling crates (compliance, perf, xtask)
docs/            mdBook documentation source
assets/          Static assets (icons, fonts, keymaps, themes, sounds)
script/          Shell/JS scripts for build, bundle, CI, release, and dev tasks
ci/              CI configuration helpers
.cargo/          Cargo config (flags, aliases)
.config/         nextest config
.github/     GitHub Actions workflows and templates
.cloudflare/     Cloudflare Workers for docs proxy and assets
.factory/        AI agent prompt templates and skills
.agents/         Agent skills (gpui-test, Sim-cherry-pick, coding)
```

## `crates/` — Key Crates

The workspace has ~200 crates. The most important ones:

| Crate | Role |
|---|---|
| `sim` | Main binary entry point, wires everything together |
| `gpui` | GPU-accelerated UI framework (rendering, layout, concurrency, events) |
| `editor` | Core editor type — text editing, LSP display, completions, inlay hints |
| `project` | File tree, worktree management, LSP client side |
| `workspace` | Window/pane/item management, local state serialization |
| `language` | Language understanding — syntax maps, symbols, LSP integration |
| `collab` | Collaboration server (runs as a separate binary) |
| `rpc` | RPC message definitions for collab |
| `agent` | Agentic AI assistant |
| `agent_ui` | UI for the agent panel |
| `language_model` | Shared LLM abstraction layer |
| `theme` | Theme system and default theme |
| `ui` | Shared UI components and patterns |
| `vim` | Vim mode implementation over `editor` |
| `lsp` | LSP server communication protocol |
| `terminal` | Terminal emulation |
| `extension_host` | WASM extension runtime |
| `db` | Local SQLite database (via `sqlez`) |
| `settings` | Settings system |
| `fs` | Filesystem abstraction |
| `text` | Rope-based text buffer |
| `multi_buffer` | Aggregated view over multiple buffers |
| `rope` | Rope data structure |
| `sum_tree` | B-tree used for buffer indexing |
| `dap` | Debug Adapter Protocol implementation |

## Conventions

- Each crate lives at `crates/<name>/` with a `Cargo.toml`. The library root is typically named after the crate (e.g., `src/editor.rs`), not `src/lib.rs`. Never use `mod.rs` — use `src/some_module.rs` instead.
- New crates: use `[lib] path = "src/<name>.rs"` in `Cargo.toml`.
- All crates declare `publish = false` (inherited from `[workspace.package]`).
- Tests live inline in the source files or in a `tests/` subdirectory within the crate.
