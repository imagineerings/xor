# Design: Rustlings CI and release

## Overview

Retain the current catalog-driven Rust product pipeline and reconcile main's automatic release implementation with the audited platform hardening. Shared CI remains a parallel worker graph with a strict aggregation job, product smoke remains downstream of that barrier, hosted Collab remains a separate PostgreSQL workflow, and Comfy backend validation remains a separate cross-platform matrix. Successful main push CI automatically prepares a semantic release, builds Linux x86_64, macOS ARM64, and Windows x86_64, then creates the tag and publishes through the single write-permission job.

The user explicitly approved automatic releases and automatic tags after the initial audit. Main's semantic-version selection, serialized release execution, stale-decision guard, tag reuse, and draft-asset verification are retained. Audited Linux compiler and Windows output-path corrections address demonstrated bundle failures. Hosted Collab remains its separate path-scoped workflow; automatic release eligibility is the successful main-push `run_tests` workflow, including product smoke and all Comfy matrix rows.

## Existing context

- `products/flavors.toml` already selects exactly `agentic-tools,rust-tools` for the Rust application and `rust-tools` for `remote_server`; `crates/product_flavor/generated_product.rs` is the generated runtime copy.
- `tooling/xtask/src/tasks/workflows/run_tests.rs` owns six parallel shared workers, a strict `always()` aggregation job, catalog-driven `product_smoke`, and the three-platform Comfy backend matrix.
- `tooling/xtask/src/tasks/workflows/hosted_collab_tests.rs` owns the separate PostgreSQL-backed Collab workflow.
- `tooling/xtask/src/tasks/workflows/release.rs` already owns a catalog-generated three-platform matrix followed by `publish_product` with the only `contents: write` permission.
- `tooling/xtask/src/tasks/bundle.rs` supplies every platform script with resolved product metadata and an isolated product output directory.
- The audited remote line adds automatic semantic-version releases, Windows Git long-path setup, isolated `cargo-about` installation, Blue Oak license policy, Visual Studio CMake selection, and Linux compiler pinning.
- The final remote release run proved that `clang-18` is absent from `ubuntu-22.04` and that an absolute product `CARGO_TARGET_DIR` becomes a `\\?\` path rejected by MSVC build scripts.

## Design decisions

### CI worker and strict aggregation graph

<!-- impl: tooling/xtask/src/tasks/workflows/run_tests.rs#release_automation_validation -->
<!-- impl: tooling/xtask/src/tasks/workflows/run_tests.rs#shared_validation -->

- Responsibility: Keep formatting/clippy, Rust-product tests, project benchmarks, Rust-tool environment tests, release-pipeline generator validation, and native Windows updater tests concurrent.
- Integration: Keep exactly one generator/products/bundle-plan validation worker in `shared_validation`; derive explicit result checks from the same worker list and keep `product_smoke` dependent on the aggregate.
- Rationale: Generator validation needs no Linux desktop dependency installation. Removing that setup reduces CI time while preserving all validation, shipped-product tests, benchmarks, and backend coverage.

### Parallel isolated benchmark configurations

<!-- impl: tooling/xtask/src/tasks/workflows/run_tests.rs#project_benchmarks -->

- Responsibility: Reduce the CI critical path without changing benchmark coverage or feature configurations.
- Integration: Expand `project_benchmarks` into two standard hosted-runner matrix rows, retaining `cargo-workspace` with `cargo_workspace` and `structured-execution` with `structured_execution`. Disable fail-fast so both configurations finish; `shared_validation` requires the complete matrix result.
- Rationale: The preceding successful main run spent 37m51s in sequential benchmarks before the 11m24s product smoke job could start. Parallel isolated runners preserve the same commands and optimization profile at the cost of one additional runner; combining feature flags or weakening coverage is unnecessary.

### Separate hosted Collab and Comfy coverage

- Responsibility: Preserve collaboration and accelerator-backend coverage outside the Rust-product package list.
- Integration: Keep `hosted_collab_tests.yml` path-scoped with its PostgreSQL service, and leave the Linux/macOS/Windows Comfy matrix unchanged.
- Rationale: These systems have platform/service requirements that do not belong in the Rust-product nextest command or its strict shared aggregation barrier.

### Automatic release and tag policy

<!-- impl: tooling/xtask/src/tasks/workflows/release.rs#release -->
<!-- impl: tooling/xtask/src/tasks/workflows/release.rs#product_builds -->
<!-- impl: tooling/xtask/src/tasks/workflows/release.rs#publish_product -->
<!-- impl: tooling/xtask/src/tasks/release_version.rs#resolve_release -->

- Responsibility: Automatically select the next semantic version after successful main push CI, build all artifacts, then create the tag and publish without partial releases.
- Integration: Preserve main's `workflow_run` trigger, tag-push/manual recovery paths, read-only preparation, serialized release group, catalog matrix, exact commit checkout, stale-decision revalidation, tag reuse, and draft asset verification.
- Rationale: Automatic tags are explicitly requested. Tag creation belongs after every platform build, not during preparation; repeated runs reuse the same commit tag and competing version decisions fail closed.

### Linux compiler selection

- Responsibility: Compile native C/C++ dependencies with a compatible hosted-runner toolchain.
- Integration: Set `CC=clang` and `CXX=clang++` on the Linux bundle step through the release generator.
- Rationale: The default GNU C++ compiler rejects the WebRTC `-Wno-changes-meaning` compatibility path, while the remote `clang-18` pin fails because that executable is not installed. The unversioned Clang selected by the repository setup exists on the runner and matches normal CI.

### Windows release hardening

<!-- impl: tooling/xtask/src/tasks/bundle.rs#product_target_dir -->
<!-- impl: script/bundle-windows.ps1 -->
<!-- impl: script/generate-licenses.ps1 -->

- Responsibility: Keep Git checkout, license generation, CMake builds, and MSVC compilation compatible with GitHub-hosted Windows.
- Integration: Enable Git long paths before bundling, isolate `cargo-about` installation from the product target, accept the demonstrated BlueOak-1.0.0 dependency license, select Visual Studio's native CMake, and pass the product target directory to Windows bundling as a repository-relative path.
- Rationale: The relative `CARGO_TARGET_DIR` still resolves to the same isolated `target/products/<id>` output and upload path, but prevents Cargo build scripts from forwarding unsupported `\\?\` file arguments to `cl.exe`.

### Native updater and release metadata validation

<!-- impl: tooling/xtask/src/tasks/workflows/run_tests.rs#windows_updater_tests -->
<!-- impl: crates/auto_update_helper/src/updater.rs#Job::mkdir -->
<!-- impl: crates/windows_resources/src/windows_resources.rs#compile -->
<!-- impl: script/bundle-mac -->
<!-- impl: script/bundle-windows.ps1 -->

- Responsibility: Catch Windows-only updater compilation/rollback regressions before release and preserve the selected product identity and version inside native bundles.
- Integration: A required Windows worker runs updater tests and checks the compiled helper's resource metadata against a synthetic release version and catalog identity. Directory rollback borrows its captured path so jobs remain reusable. Windows resource compilation consumes the bundle plan's display name, icon set, and release version with existing development fallbacks. macOS asks cargo-bundle for only its intermediate `.app`, requires exactly one resulting application path, and then creates the catalog-named DMG itself; it sets both plist version keys from `RELEASE_VERSION` before signing. Native bundlers assert metadata before packaging.
- Artifact verification: Before upload, each native job runs `script/smoke-product-bundle` using the resolved catalog plan. Linux extracts its archive; macOS mounts/copies its DMG and verifies plist/signature metadata; Windows silently installs and uninstalls its installer. Each runs the installed CLI version and editor help startup. Windows additionally runs foreground system-specs startup with temporary user data. Its foreground argument and console attachment remain available independently of Comfy, preserving the CLI and single-instance contract with the exact Rust-product feature set. This does not certify a graphical session. The Linux, macOS, and Windows CLIs use catalog branding; no platform hardcodes the upstream product name in version output. Application version loading honors the compiled release version before falling back to the crate version.
- Rationale: Run 33374877495 failed in Windows-only code that Linux tests cannot compile. Its macOS job also reported executable and archive I/O failures; the release generator records architecture, toolchain, and disk usage to distinguish runner failures from source defects without suppressing errors or changing the build profile.

### Parallel native compilation and packaging

<!-- impl: tooling/xtask/src/tasks/workflows/release.rs#application_builds -->
<!-- impl: tooling/xtask/src/tasks/workflows/release.rs#remote_server_builds -->
<!-- impl: tooling/xtask/src/tasks/workflows/release.rs#product_builds -->
<!-- impl: tooling/xtask/src/tasks/bundle.rs#BundlePhase -->
<!-- impl: script/bundle-linux -->
<!-- impl: script/bundle-mac -->
<!-- impl: script/bundle-windows.ps1 -->

- Responsibility: Remove the sequential remote-server compilation from each platform's critical path without changing the compiled packages, release profile, catalog features, signing, smoke checks, or final artifacts.
- Integration: The preparation job resolves the release with `xtask`, emits commit-bound dry-run plans for all three targets from that same binary, and uploads them once. One Linux job installs pinned prebuilt `cargo-about` and generates `licenses.md` from the selected commit and `Cargo.lock`. Three application builds consume that identical file while three remote-server builds run independently. Both matrices upload raw binaries plus metadata recording the release commit, exact target, and catalog-selected features. Three final native jobs verify the plan, license, and binary metadata, restore the inputs to their normal product target paths, run the platform bundlers in package-only mode, sign where credentials are available, execute the existing smoke checks without recompiling `xtask`, and upload the unchanged catalog-named bundles. The macOS package job caches only the versioned `cargo-bundle` executable under a dedicated tool key. The publisher depends only on the complete final packaging matrix and downloads only those three release artifacts.
- Rationale: In release run 33799119392, the application builds took about 60 minutes on Linux and 108 minutes on macOS and Windows, after which remote-server builds added about 28, 46, and 47 minutes respectively. Separate hosted runners overlap those independent compilations. Explicit artifact metadata prevents a package job from combining inputs from a different commit, target, or feature plan.

### GitHub-managed compiler object caching

<!-- impl: tooling/xtask/src/tasks/workflows/release.rs#configure_release_cache_namespace -->
<!-- impl: tooling/xtask/src/tasks/workflows/steps.rs#setup_release_sccache -->
<!-- impl: tooling/xtask/src/tasks/workflows/steps.rs#finalize_release_sccache -->

- Responsibility: Reuse unchanged release-mode Rust compilation without archiving complete product target directories or exposing private cache credentials.
- Integration: The pinned Mozilla `sccache` action supplies the GitHub Actions object backend. Every compile row enables the backend with a namespace containing an explicit schema version, runner operating system, exact compilation target, and the repository toolchain-file hash. The cache daemon has no idle timeout during long optimized crates. Strict finalizers print statistics to the log and job summary before stopping the server; missing objects perform the unchanged cold build. Package-only rows perform no Rust compilation and record zero cache requests in their summaries. Existing R2 setup remains available to unrelated workflows but is not required by the public release pipeline.
- Rationale: The 3 GiB directory caches in release run 33799119392 reported only 0.23% Linux, 0.16% macOS, and 0.17% Windows hit rates while the macOS product target reached 28 GiB. Object-level storage avoids transferring one truncated directory archive and retains useful compiler results independently.

### Generator and catalog ownership

<!-- impl: products/flavors.toml -->
<!-- impl: crates/product_flavor/generated_product.rs -->

- Responsibility: Prevent YAML and runtime metadata drift.
- Integration: Change Rust generator/catalog consumers first, regenerate with `cargo xtask products` only if catalog output changes, then run `cargo xtask workflows` and inspect the generated YAML.
- Rationale: `products/flavors.toml`, the generated product table, and the workflow generators remain the reviewable sources of truth.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2, 1.3, 1.4 | CI worker and strict aggregation graph | Generator tests and generated dependency inspection |
| 1.5 | Separate hosted Collab and Comfy coverage | Hosted workflow generator test and YAML inspection |
| 1.6, 1.7 | Separate hosted Collab and Comfy coverage | Comfy matrix assertions and forbidden-reference scans |
| 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.9 | Automatic release and tag policy; generator and catalog ownership | Release/version tests, matrix/artifact/permission inspection, bundle dry runs |
| 2.10 | CI worker and strict aggregation graph; parallel isolated benchmark configurations | One lightweight validation worker, no desktop setup, exact benchmark matrix rows, all original worker coverage retained |
| 2.11 | GitHub-managed compiler object caching | Generated namespaces/backend environment, strict job-summary statistics, and consecutive native release runs |
| 2.12 | Parallel native compilation and packaging | Generated job dependencies, intermediate metadata checks, final native smoke tests, and release artifacts |
| 1.8, 3.5 | Native updater and release metadata validation | Windows updater tests, compiled resource check, plist assertions |
| 2.8 | Linux compiler selection; Windows release hardening | Generated environment assertions and Windows bundle regression tests |
| 3.1, 3.2, 3.3, 3.4 | Generator and catalog ownership | Product check, generated metadata comparison, platform bundle-plan dry runs |
| 4.1, 4.2, 4.3, 4.4 | Generator and catalog ownership | Workflow generation, check-workflows, full xtask tests |
| 5.1, 5.2, 5.3, 5.4 | Evidence-based audit and selective implementation | Git diff/commit inspection, GitHub check/run evidence, focused final diff review |
| 5.5 | Guarded post-merge cleanup | Committed branch-audit record, fresh remote/PR inspection, exact-SHA deletion leases |

## Testing strategy

<!-- impl: .agents/specs/rustlings-ci-release/branch-audit.md -->

- Validate this spec in canonical mode before and after implementation.
- Run focused workflow-generator and bundle regression tests, then full `cargo test -p xtask`.
- Run `cargo xtask products --check`, regenerate workflows, and run `cargo xtask check-workflows`.
- Parse and inspect all active generated YAML for triggers, runner labels, permissions, concurrency, dependencies, feature lists, platform targets, artifact paths, and forbidden infrastructure.
- Run Linux/macOS shell syntax and product bundle dry runs plus the Windows dry-run/regression assertions available from the macOS host.
- Run the exact Rust-product release check, warning-denied Clippy for changed Rust crates, full repository Clippy, formatting, and diff checks.
