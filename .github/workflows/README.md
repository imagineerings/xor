# Validation scope

The required `run_tests` workflow validates the enabled Copper (`rust`) product rather than every test target in the repository. Its `copper_tests` job is generated from the product manifest features and an explicit package list in `tooling/xtask/src/tasks/workflows/run_tests.rs`.

## Copper releases

Copper is the only product currently enabled in `products/flavors.toml`. The
generated `release.yml` has three supported entry points:

- a successful completion of `run_tests` on `main` from this repository;
- manual dispatch for a patch, minor, major, or explicit recovery version;
- a manually created `rust-v*` tag that points to a commit contained in
  `main`.

The automatic path checks out `workflow_run.head_sha`, so it packages the exact
commit that passed CI. If there is no existing `rust-v*` tag, the first release
uses `crates/zed/Cargo.toml`; subsequent automatic releases increment the patch
component. Tags use `rust-vX.Y.Z`, and releases are titled `Copper X.Y.Z`.

The workflow resolves the version, builds Linux x86_64, macOS ARM64, and
Windows x86_64 artifacts, and waits for every matrix entry. The final job then
fetches tags again, repeats the version/history checks, validates exactly one
artifact per target against the product manifest, creates an annotated tag,
and creates or updates the GitHub Release. A failed or cancelled test run,
fork-originated run, platform build failure, version conflict, or stale release
decision cannot publish a tag or release. Workflow concurrency serializes the
entire release operation without cancelling an active release.

Request a manual release from the default branch with:

```sh
gh workflow run release.yml --ref main -f bump=minor
gh workflow run release.yml --ref main -f bump=major
```

For recovery, `version` may contain an exact stable `X.Y.Z` and `commit_sha`
may identify a full commit SHA contained in `main`:

```sh
gh workflow run release.yml --ref main \
  -f bump=patch \
  -f version=<X.Y.Z> \
  -f commit_sha=<FULL_COMMIT_SHA>
```

Rerunning a completed or interrupted release is safe: an existing tag is reused
only when it already points to the selected commit, an existing release is
updated, and missing assets are uploaded without duplicating existing assets.
A tag conflict fails visibly and is never moved. The final publication job alone has
`contents: write`; preparation and build jobs have `contents: read`. New
releases remain drafts until the remote asset list exactly matches the three
validated local artifacts.

Signing retains the product bundler's `auto` policy. Signed macOS artifacts
require `MACOS_SIGNING_IDENTITY`, `MACOS_CERTIFICATE`,
`MACOS_CERTIFICATE_PASSWORD`, `APPLE_NOTARIZATION_KEY`,
`APPLE_NOTARIZATION_KEY_ID`, and `APPLE_NOTARIZATION_ISSUER_ID`. Signed Windows
artifacts require `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`,
`AZURE_CLIENT_SECRET`, `ACCOUNT_NAME`, `CERT_PROFILE_NAME`, `ENDPOINT`,
`FILE_DIGEST`, `TIMESTAMP_DIGEST`, and `TIMESTAMP_SERVER`. When a complete set
is unavailable, the current `auto` policy produces an ad-hoc-signed macOS
bundle or unsigned Windows bundle; it does not fail a stable release.

The required desktop collaboration baseline includes `client`, `call`, `channel`, and `collab_ui` without `multiplayer-tools`. These packages provide application connectivity, calls, channels, and collaboration UI used by the Rust product even when its multiplayer workspace is not compiled.

## Deferred suites

Deferred suites are not counted as passing when they do not run.

### Multiplayer desktop behavior

`multiplayer-tools` behavior is outside the current Rust product because `products/flavors.toml` enables exactly `agentic-tools,rust-tools`. When that product enables `multiplayer-tools`, add its desktop tests to the required product gate using:

```sh
ZED_PRODUCT_ID=rust cargo nextest run --no-default-features \
  --features zed/agentic-tools,zed/rust-tools,zed/multiplayer-tools,channel/multiplayer-tools,collab_ui/multiplayer-tools \
  -p zed -p channel -p collab_ui
```

### Hosted Collab server

The `collab` package is the hosted collaboration server. Its `collab_tests` integration target requires the `test-support` feature and contains both portable tests using their intended default backend and tests that explicitly select PostgreSQL. The generated `hosted_collab_tests` workflow provides PostgreSQL 15 through `COLLAB_TEST_DATABASE_URL` without globally overriding the backend selected by each test, and runs manually or whenever hosted collaboration paths change:

```sh
cargo nextest run --package collab --features test-support --test collab_tests \
  --no-fail-fast --no-tests=warn
```

This workflow becomes a required product gate if Copper enables `multiplayer-tools`, operates its own Collab service, or otherwise makes hosted collaboration part of the shipped product. It remains a separate PostgreSQL-backed gate rather than being mixed into lightweight desktop product tests.

### Comfy runtime and evidence

The Linux, macOS, and Windows Comfy backend warning-denied Clippy matrix remains required. Comfy runtime and evidence tests are deferred until Comfy becomes a shipped product feature and CI has the pinned source/evidence fixtures plus the filesystem and runtime resources those suites certify. Restore them as a dedicated job with:

```sh
cargo nextest run --no-fail-fast --no-tests=warn \
  -p comfy_api -p comfy_media -p comfy_model -p comfy_nodes \
  -p comfy_plugin_host -p comfy_plugin_sdk -p comfy_plugin_worker_fixture \
  -p comfy_runtime -p comfy_sampler -p comfy_tensor -p comfy_test_support \
  -p comfy_types -p comfy_ui -p comfy_worker
```

The dedicated job must provision the repository-pinned Comfy source/evidence fixture trees, filesystem space, and any certified runtime resources before this command becomes required.
