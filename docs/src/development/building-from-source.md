---
title: Building from Source
description: Set up a local Sim development environment and build the app from source.
---

# Building from Source

Sim is a Rust workspace with a GPUI desktop application, supporting crates, extensions, and documentation tooling. Use the pinned toolchain and repository scripts rather than ad hoc local setup where possible.

## Prerequisites

- Rust toolchain from `rust-toolchain.toml`.
- Platform build tools for macOS, Linux, or Windows.
- Git.
- Node.js only for docs or frontend-adjacent package work.

On macOS, start with:

```sh
./script/bootstrap
```

## Build

Run the desktop app in debug mode:

```sh
cargo run
```

Run a release build when you need production-like performance:

```sh
cargo run --release
```

## Validate

Before sending a change for review, run the narrowest validation that covers your change. Common commands:

```sh
./script/clippy
cargo nextest run -p <crate-name> --no-fail-fast
cargo fmt --all
```

Use workspace-wide validation for broad changes or shared APIs.

## Documentation

The existing docs tree is mdBook-based and the migration adds Docusaurus scaffolding. For docs-only changes, validate links and sidebar entries relevant to the touched pages.
