# Implementation Plan: Rustlings CI and release

## Approach

First establish stable Rustlings package naming, then replace the active CI and release compositions without deleting helper APIs used by archived workflows. Regenerate and validate the YAML only after the generator and packaging sources agree on artifact paths.

## Tasks

### Milestone 1: Minimal Rustlings automation

- [x] 1. Deliver focused generated workflows and packages
  - [x] 1.1. Brand stable platform bundles as Rustlings
    - Preserve internal `zed` crate and executable paths while changing installed display names and final package filenames.
    - Preserve optional signing and unsigned/ad-hoc fallback behavior.
    - _Requirements: 3.1, 3.2, 3.3, 3.4_
    - _Depends on: none_
    - _Reads: crates/zed/Cargo.toml, script/bundle-linux, script/bundle-mac, script/bundle-windows.ps1_
    - _Writes: crates/zed/Cargo.toml, script/bundle-linux, script/bundle-mac, script/bundle-windows.ps1_
    - _Validation: run Linux and macOS bundle dry-run modes, inspect the Windows dry-run and stable installer branches, validate shell syntax, and inspect stable packaging metadata_
    - _Evidence: Linux and macOS dry runs reported `rust_tools=true`; `bash -n script/bundle-linux script/bundle-mac` and Cargo metadata parsing passed; Windows stable and early dry-run branches were inspected because `pwsh` is unavailable on the macOS host._
  - [x] 1.2. Replace the active CI composition with one GitHub-hosted validation job
    - Retain generator helpers used outside the active `run_tests()` entry point.
    - Include formatting, repository clippy, cargo-nextest, and an explicit `agentic,rust-tools` release check.
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 4.1_
    - _Depends on: none_
    - _Reads: tooling/xtask/src/tasks/workflows/run_tests.rs, tooling/xtask/src/tasks/workflows/steps.rs_
    - _Writes: tooling/xtask/src/tasks/workflows/run_tests.rs, .github/workflows/run_tests.yml_
    - _Validation: regenerate workflows and run the focused Rustlings CI generator test covering triggers, runner, steps, concurrency, and forbidden references_
    - _Evidence: `cargo xtask workflows`, `cargo xtask check-workflows`, and `cargo test --package xtask rustlings_` passed; generated CI has one Ubuntu job and the requested checks with no forbidden private infrastructure._
  - [x] 1.3. Replace the active release composition with three bundle jobs and one publish job
    - Use standard hosted runners and existing bundle scripts with Rust tools enabled.
    - Normalize platform packages to Rustlings artifact names and publish only behind an all-builds dependency barrier.
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.2, 4.1_
    - _Depends on: 1.1_
    - _Reads: tooling/xtask/src/tasks/workflows/release.rs, tooling/xtask/src/tasks/workflows/steps.rs, script/bundle-linux, script/bundle-mac, script/bundle-windows.ps1_
    - _Writes: tooling/xtask/src/tasks/workflows/release.rs, .github/workflows/release.yml_
    - _Validation: regenerate workflows and run the focused Rustlings release generator test covering triggers, permissions, runners, bundle commands, artifacts, publish dependencies, and forbidden references_
    - _Evidence: `cargo xtask workflows`, `cargo xtask check-workflows`, and `cargo test --package xtask rustlings_` passed; the generated publish job alone has `contents: write` and needs all three hosted-runner build jobs._
  - [x] 1.4. Validate generated workflow integrity
    - Run focused generator tests, canonical workflow generation/validation, and static YAML checks.
    - Reconcile task evidence with the final changed paths and results.
    - _Requirements: 4.1, 4.2_
    - _Depends on: 1.2, 1.3, 1.5_
    - _Reads: tooling/xtask/src/tasks/workflows.rs, .github/workflows/release.yml, .github/workflows/run_tests.yml_
    - _Writes: .agents/specs/rustlings-ci-release/tasks.md_
    - _Validation: run the canonical spec validator, focused Rustlings generator tests, `cargo xtask workflows`, repository workflow checks, formatting, and diff checks_
    - _Evidence: canonical spec validation, both Rustlings generator tests, workflow generation/validation, `cargo fmt --all -- --check`, `git diff --check`, and focused xtask clippy passed. The pre-existing archive-policy test still fails because `HEAD` lacks `.github/workflows/archive/bump_zed_version.yml`; it passes the active-workflow assertion before reaching that unrelated missing-file assertion._
  - [x] 1.5. Remove generator code made obsolete by the active workflow simplification
    - Remove private job constructors and builder options that no generated workflow consumes.
    - Preserve helpers still referenced by archived, nightly, compliance, and extension workflow generators.
    - _Requirements: 1.4, 2.6, 4.2_
    - _Depends on: 1.2, 1.3_
    - _Reads: tooling/xtask/src/tasks/workflows.rs, tooling/xtask/src/tasks/workflows/deploy_docs.rs, tooling/xtask/src/tasks/workflows/steps.rs, tooling/xtask/src/tasks/workflows/vars.rs_
    - _Writes: tooling/xtask/src/tasks/workflows.rs, tooling/xtask/src/tasks/workflows/deploy_docs.rs, tooling/xtask/src/tasks/workflows/steps.rs, tooling/xtask/src/tasks/workflows/vars.rs_
    - _Validation: run focused xtask clippy with warnings denied and regenerate every workflow successfully_
    - _Evidence: `./script/clippy --no-all-features --package xtask` passed with warnings denied, and `cargo xtask workflows` regenerated all configured workflows successfully._
