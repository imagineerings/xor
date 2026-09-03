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

### Milestone 3: Approved automatic release reconciliation

- [x] 3. Reconcile main with automatic releases and efficient CI
  - [x] 3.1. Preserve automatic semantic tags and audited bundle fixes
    - Resolve generator and packaging conflicts semantically, retaining main's automatic release preparation/publication and the audited hosted-runner corrections.
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 3.1, 3.2, 3.3, 3.4, 5.4_
    - _Depends on: 2.5_
    - _Reads: tooling/xtask/src/tasks/release_version.rs, tooling/xtask/src/tasks/workflows/release.rs, tooling/xtask/src/tasks/bundle.rs_
    - _Writes: tooling/xtask/src/tasks/workflows/release.rs, tooling/xtask/src/tasks/bundle.rs, script/bundle-windows.ps1, script/generate-licenses.ps1_
    - _Validation: release/version/bundle tests, exact product release check, three bundle plans, focused and full Clippy_
    - _Evidence: All 28 xtask tests, all three bundle plans, the exact Rust product release check, focused warning-denied xtask Clippy, and full workspace Clippy passed. Automatic tag/version/retry behavior is retained; native cross-platform release execution remains an automatic post-merge check, not a locally dispatched operation._
  - [x] 3.2. Reconcile concurrent validation and regenerate workflows
    - Retain one lightweight generator validation worker and all existing product, collaboration, benchmark, Rust-tools, and Comfy coverage.
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 2.10, 4.1, 4.2, 4.3, 4.4_
    - _Depends on: 3.1_
    - _Reads: tooling/xtask/src/tasks/workflows/run_tests.rs, tooling/xtask/src/tasks/workflows/hosted_collab_tests.rs_
    - _Writes: tooling/xtask/src/tasks/workflows/run_tests.rs, .github/workflows/run_tests.yml, .github/workflows/release.yml_
    - _Validation: full xtask tests, product check, workflow generation/freshness, formatting, and YAML inspection; PR CI is the external landing gate below_
    - _Evidence: Full xtask tests, catalog check, workflow generation/checks, formatting, Bash syntax, and YAML parsing passed. Executed the generated strict barrier with all-success and each worker individually failed/cancelled/skipped; only all-success passed. The single generator-validation worker no longer installs desktop Linux dependencies._
  - [x] 3.3. Record complete branch dispositions and guarded cleanup eligibility
    - Preserve branches with valuable unmerged work or relevant open PRs; perform authorized remote cleanup only after the reconciled PR is merged.
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_
    - _Depends on: 3.2_
    - _Reads: origin/codex/*, origin/main, .agents/specs/rustlings-ci-release/branch-audit.md_
    - _Writes: .agents/specs/rustlings-ci-release/branch-audit.md_
    - _Validation: exact tree comparisons, branch/PR inventory, fresh inspected remote tips, and a recorded expected-SHA deletion safeguard; actual cleanup follows the external landing gate_
    - _Evidence: Refreshed all origin branches; all five codex tips exactly match their merged main squash trees, and PRs 6–10 are merged with no open codex PRs. Full recovery SHAs, dependencies, semantic adaptations, and conditional deletion rationales are recorded in branch-audit.md._

  - [x] 3.4. Parallelize isolated benchmark configurations without reducing coverage
    - Keep both benchmark targets, exact feature configurations, and optimized profiles while running them on separate hosted-runner matrix rows.
    - _Requirements: 1.1, 1.3, 1.4, 2.10, 4.1, 4.3, 4.4_
    - _Depends on: 3.2_
    - _Reads: tooling/xtask/src/tasks/workflows/run_tests.rs, crates/project/Cargo.toml_
    - _Writes: tooling/xtask/src/tasks/workflows/run_tests.rs, .github/workflows/run_tests.yml_
    - _Validation: focused/full xtask tests, generated matrix and strict barrier inspection, workflow freshness, formatting, focused/full Clippy_
    - _Evidence: All 28 xtask tests, workflow regeneration/checks, product check, formatting, focused/full warning-denied Clippy, and parsed YAML inspection passed. Both exact feature/benchmark pairs are separate matrix rows with fail-fast disabled; the complete matrix remains required by shared_validation._

## Authorized external landing and cleanup

After committing the local reconciliation, require PR #11 checks to pass and verify its merge before deleting any eligible remote branch. Immediately before each deletion, recheck the remote tip and relevant open PRs and use the exact recorded SHA as a deletion lease. Preserve changed tips for re-audit. Report merge/check evidence and actual deletion results separately; do not manually dispatch releases, alter protections, or delete main/dev/rustlings, tags, or local branches.


### Milestone 4: Repair, verify, and accelerate native releases

- [x] 4. Repair demonstrated native failures, verify publication, and reuse compilation
  - [x] 4.1. Fix reusable updater rollback and native release metadata
    - Add native Windows updater tests to the required CI graph; propagate resolved product/version metadata into Windows resources and macOS plists; restore Windows foreground startup without Comfy; retain optional signing and collect macOS runner diagnostics.
    - _Requirements: 1.8, 2.1, 2.2, 2.4, 2.5, 2.7, 2.9, 3.5, 4.1_
    - _Depends on: 3.4_
    - _Reads: products/flavors.toml, tooling/xtask/src/tasks/bundle.rs, crates/auto_update_helper/src/updater.rs_
    - _Writes: crates/auto_update_helper/src/updater.rs, crates/windows_resources/src/windows_resources.rs, crates/cli/src/main.rs, crates/zed/src/main.rs, crates/release_channel/src/lib.rs, script/smoke-product-bundle, script/bundle-mac, script/bundle-windows.ps1, tooling/xtask/src/tasks/workflows/run_tests.rs, tooling/xtask/src/tasks/workflows/release.rs, .github/workflows/run_tests.yml, .github/workflows/release.yml_
    - _Validation: formatting, xtask tests, workflow freshness, product catalog check, targeted Clippy, native Windows CI, macOS build-script/plist checks, all three release bundles, automatic tag and published asset inspection_
    - _Evidence: Fixes landed through PRs 12-17. Release run 33657478916 passed Linux x86_64, macOS ARM64, Windows x86_64, and gated publication from commit `54ef39a5f2c3f53b7558c8e999161047bd5e97c8`; annotated tag `rust-v1.16.2` and the non-draft Copper 1.16.2 release contain exactly three assets that matched their workflow artifacts byte-for-byte._
  - [x] 4.2. Reuse bounded release-mode compilation across native platforms
    - Add a GitHub-managed local `sccache` backend to the generated release matrix without exposing R2 credentials or changing the native bundle commands.
    - Isolate cache entries by operating system, target triple, and toolchain; bound every platform cache; preserve cold-build fallback and emit final cache statistics.
    - _Requirements: 2.1, 2.2, 2.4, 2.5, 2.6, 2.7, 2.11, 4.1, 4.3, 4.4_
    - _Depends on: 4.1_
    - _Reads: tooling/xtask/src/tasks/workflows/release.rs, tooling/xtask/src/tasks/workflows/steps.rs, script/setup-sccache, script/setup-sccache.ps1_
    - _Writes: tooling/xtask/src/tasks/workflows/release.rs, tooling/xtask/src/tasks/workflows/steps.rs, script/setup-sccache, script/setup-sccache.ps1, .github/workflows/release.yml_
    - _Validation: prove a local release-mode cache hit with clean target output, run script syntax checks, full xtask tests and Clippy, regenerate and validate workflows, inspect permissions/features/matrix/publish fan-in, then verify the automatic native release and cache statistics after merge_
    - _Evidence: A local disk backend with two identical release-mode Cargo builds and a cleaned target produced one miss followed by one Rust cache hit. The generated workflow uses a pinned GitHub cache action, 3 GiB per-platform bounds, target/toolchain keys, read-only native permissions, unchanged hosted runners and bundle commands, strict statistics, and no R2 credentials._
  - [x] 4.3. Keep the release cache daemon alive during long crate compilations
    - Disable the local release cache daemon's idle timeout in the workflow generator and persist the setting across steps in both setup scripts.
    - Keep cache statistics and shutdown strict so a cache failure remains visible, and leave existing R2-backed CI behavior unchanged.
    - _Requirements: 2.6, 2.11, 4.1, 4.3, 4.4_
    - _Depends on: 4.2_
    - _Reads: tooling/xtask/src/tasks/workflows/steps.rs, script/setup-sccache, script/setup-sccache.ps1_
    - _Writes: tooling/xtask/src/tasks/workflows/steps.rs, script/setup-sccache, script/setup-sccache.ps1, .github/workflows/release.yml_
    - _Validation: regenerate and validate workflows, run script syntax checks and xtask tests, and verify a Windows hosted release bundle outlives the prior ten-minute single-crate boundary_
    - _Evidence: Release run 33736625225 began the Windows `project` crate at 11:34:59 and lost the sccache server at 11:45:24 with Windows socket error 10054; upstream documents a ten-minute daemon idle timeout and `SCCACHE_IDLE_TIMEOUT=0` as the persistent-server setting._

The 2026-08-31 release-repair request separately authorizes scoped commits, pushes, PRs, merging after checks pass, and release reruns through successful publication. It supersedes the earlier restriction on manually dispatching releases for this repair only. Existing tags and published assets must not be moved, deleted, or overwritten; unrelated branch cleanup is postponed.
