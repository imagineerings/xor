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
    - Include formatting, repository clippy, cargo-nextest, and an explicit `multiplayer-tools,rust-tools` release check.
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

### Milestone 2: Evidence-based pipeline reconciliation

- [x] 2. Reconcile remote CI and release work with the catalog-driven pipeline
  - [x] 2.1. Audit and disposition every related remote `codex/*` branch
    - Inspect unique commits, resulting diffs, overlap, dependencies, GitHub checks, and actual release-job outcomes.
    - Record equivalent/superseding commit trees and reject unrelated README changes or unapproved release-policy expansion.
    - _Requirements: 5.1, 5.2, 5.3, 5.4_
    - _Depends on: none_
    - _Reads: origin/codex/*, tooling/xtask/src/tasks/workflows/, .github/workflows/, products/flavors.toml_
    - _Writes: .agents/specs/rustlings-ci-release/tasks.md_
    - _Validation: compare every related branch to rustlings with git log/diff and inspect GitHub check and release-run evidence_
    - _Evidence: Compared all five remote `codex/*` branches and their unique commits/files to `rustlings`; verified duplicate commit trees, ordinary CI results, and each automatic release attempt. The final Linux run failed on absent `clang-18`, and the final Windows run failed when MSVC received an extended-length absolute product target path._
  - [x] 2.2. Add concurrent release-pipeline validation without changing release policy
    - Reuse the existing strict aggregation result check and preserve the Rust-product test package boundary.
    - Keep hosted Collab and Comfy backend validation separate and intact.
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 4.1, 4.4_
    - _Depends on: 2.1_
    - _Reads: tooling/xtask/src/tasks/workflows/run_tests.rs, tooling/xtask/src/tasks/workflows/hosted_collab_tests.rs_
    - _Writes: tooling/xtask/src/tasks/workflows/run_tests.rs_
    - _Validation: run focused run_tests and hosted_collab_tests generator tests and inspect strict dependencies/features_
    - _Evidence: Added the independent generator/products/bundle-plan worker to the strict aggregation dependency set. Focused and full xtask tests passed; generated YAML retains the shipped-product package boundary, separate hosted Collab workflow, complete Comfy matrix, and `product_smoke` dependency on the strict barrier._
  - [x] 2.3. Reconcile release generator hardening with hosted-runner toolchains
    - Enable Windows Git long paths and select the available unversioned Clang toolchain for Linux bundles.
    - Preserve the tag/manual trigger, catalog matrix, minimal permissions, optional signing, and all-build publish barrier.
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 4.1, 4.4_
    - _Depends on: 2.1_
    - _Reads: tooling/xtask/src/tasks/workflows/release.rs, tooling/xtask/src/tasks/workflows/steps.rs_
    - _Writes: tooling/xtask/src/tasks/workflows/release.rs_
    - _Validation: run focused release generator tests and inspect generated Linux/Windows steps, permissions, matrix, and fan-in_
    - _Evidence: Generated release tests and YAML inspection confirm tag/manual triggers, three standard hosted runners, optional signing inputs, Windows long paths, unversioned Linux Clang, one write-permission publish job, and the complete matrix fan-in._
  - [x] 2.4. Harden Windows product bundling at the demonstrated failure boundaries
    - Isolate cargo-about installation, use Visual Studio CMake, accept the required Blue Oak license, and keep MSVC build paths repository-relative.
    - Preserve the catalog-owned output path and internal Zed package names.
    - _Requirements: 2.2, 2.3, 2.7, 2.8, 3.1, 3.3, 3.4, 5.3, 5.4_
    - _Depends on: 2.1_
    - _Reads: tooling/xtask/src/tasks/bundle.rs, script/bundle-windows.ps1, script/generate-licenses.ps1, script/licenses/zed-licenses.toml_
    - _Writes: tooling/xtask/src/tasks/bundle.rs, script/bundle-windows.ps1, script/generate-licenses.ps1, script/licenses/zed-licenses.toml_
    - _Validation: run xtask bundle tests, Windows bundle dry-run regression assertions, and inspect resolved output and compiler paths_
    - _Evidence: Focused regression tests passed for the relative Windows target path, temporary cargo-about target, and Visual Studio CMake selection. The Windows product dry run resolves `target/products/rust`; native Windows execution remains a generated CI responsibility because PowerShell/MSVC are unavailable on the macOS host._
  - [x] 2.5. Regenerate and validate product and workflow artifacts
    - Confirm the Rust catalog and generated metadata retain exactly `agentic-tools,rust-tools` plus remote `rust-tools`.
    - Regenerate workflow YAML from the reconciled generators and run the full requested quality gate.
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4_
    - _Depends on: 2.2, 2.3, 2.4_
    - _Reads: products/flavors.toml, crates/product_flavor/generated_product.rs, tooling/xtask/src/tasks/workflows/, script/bundle-linux, script/bundle-mac, script/bundle-windows.ps1_
    - _Writes: .github/workflows/run_tests.yml, .github/workflows/release.yml, .github/workflows/hosted_collab_tests.yml, .agents/specs/rustlings-ci-release/tasks.md_
    - _Validation: run the canonical spec validator, cargo fmt, focused and full xtask tests, products check, workflow generation/checks, bundle dry runs, exact Rust release check, focused and full Clippy, YAML inspection, and git diff checks_
    - _Evidence: On 2026-08-30, both canonical spec validations, `cargo fmt --all -- --check`, focused and full `cargo test -p xtask`, `cargo xtask products --check`, workflow regeneration/checks, Ruby YAML parsing and manual YAML inspection, Linux/macOS shell syntax, three bundle dry runs, the exact release-profile Rust product check, focused/full warning-denied Clippy, and diff checks passed._
