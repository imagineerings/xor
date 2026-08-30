# Requirements: Rustlings CI and release

## Problem

The Rust product shares a large Zed workspace, but its CI and releases must remain usable on a public fork without private runners, caches, organization secrets, or manually synchronized product data. Several remote `codex/*` branches contain overlapping release automation and platform fixes, including fixes whose CI checks passed even though real release bundles still failed.

## Scope

### In scope

- Audit every remote CI, release, packaging, product-flavor, GitHub Actions, and test-performance branch against `rustlings` using commit contents and actual workflow evidence.
- Preserve the catalog-selected Rust application features `agentic-tools,rust-tools` and remote-server feature `rust-tools`.
- Keep shared validation, focused Rust-product tests, hosted Collab tests, Comfy backend checks, product smoke validation, and cross-platform release packaging on standard GitHub-hosted runners.
- Integrate only self-contained or semantically corrected remote changes whose resulting behavior is maintainable and supported by evidence.
- Keep generated workflows owned by `tooling/xtask` and product identity owned by `products/flavors.toml` plus generated product metadata.

### Out of scope

- Merging remote `codex/*` branches wholesale.
- Renaming internal Zed crates, modules, packages, protocols, or project-local `.zed` paths.
- Requiring signing credentials, manually deploying a release during implementation, or deleting remote branches.
- Removing coverage to reduce CI duration.

## Requirements

### Requirement 1: Strict public-fork continuous integration

**User story:** As a fork maintainer, I want concurrent focused validation with one strict result, so that failures are visible without private infrastructure or unnecessary whole-workspace testing.

#### Acceptance criteria

1. **1.1** WHEN a pull request, a push to `main`, or a manual dispatch occurs, THEN THE generated CI SHALL run independent validation workers on standard GitHub-hosted runners and cancel superseded runs for the same non-main ref.
2. **1.2** THE Rust-product test worker SHALL run the shipped Rust product package set with exactly `zed/agentic-tools`, `zed/rust-tools`, and `remote_server/rust-tools`, including baseline `client`, `call`, `channel`, and `collab_ui` coverage, rather than testing the entire workspace.
3. **1.3** THE strict aggregation job SHALL run after every shared worker regardless of outcome and SHALL fail when any worker failed, was cancelled, or did not complete successfully.
4. **1.4** THE `product_smoke` job SHALL depend on the strict aggregation job and SHALL derive application and remote-server features from the enabled product catalog.
5. **1.5** THE hosted Collab test workflow SHALL remain separate, path-scoped, PostgreSQL-backed, and runnable on a standard GitHub-hosted Linux runner.
6. **1.6** THE Linux, macOS, and Windows Comfy backend validation rows SHALL remain intact and independent of Rust-product smoke validation.
7. **1.7** THE active CI SHALL NOT require an organization-owner guard, Namespace or self-hosted runner, external cache, Slack, Sentry, or unrelated organization secret.

### Requirement 2: Catalog-driven cross-platform release

**User story:** As a release maintainer, I want one generated product matrix with a strict publish barrier, so that every Copper release contains the supported Rust-product bundles and no partial release is published.

#### Acceptance criteria

1. **2.1** WHEN main push CI succeeds, a `rust-v*` tag is pushed, or a manual recovery dispatch starts the release workflow, THEN THE workflow SHALL build Linux x86_64, macOS ARM64, and Windows x86_64 bundles on matching standard GitHub-hosted runners.
2. **2.2** EVERY platform bundler SHALL receive exactly the catalog-selected application features `agentic-tools,rust-tools` and remote-server feature `rust-tools` through `cargo xtask bundle --product rust`.
3. **2.3** THE platform matrix, product identity, installer names, and release artifact names SHALL derive from the validated Rust product catalog and generated product metadata.
4. **2.4** THE publishing job SHALL depend on the complete platform matrix, validate one artifact per supported target, and SHALL NOT run successfully after any failed or cancelled platform build.
5. **2.5** THE workflow SHALL use read-only permissions by default and grant `contents: write` only to the publishing job.
6. **2.6** THE release workflow SHALL NOT require an organization-owner guard, Namespace or self-hosted runner, external cache, Slack, Sentry, compliance service, hard-coded external repository, or unrelated organization secret.
7. **2.7** IF signing credentials are absent, THEN the platform bundle plan SHALL retain unsigned or ad-hoc output; no credentials SHALL be embedded in repository data or generated YAML.
8. **2.8** THE Linux and Windows bundle environments SHALL select native compilers that exist on their hosted runners and SHALL avoid passing unsupported extended-length output paths to MSVC build scripts.
9. **2.9** WHEN all platform builds succeed for an automatically selected main commit, THEN THE publisher SHALL create the next semantic `rust-vX.Y.Z` tag and release automatically; reruns SHALL reuse an existing matching commit tag, and stale or conflicting release decisions SHALL fail before publication.
10. **2.10** THE CI SHALL retain concurrent focused workers, one generator-validation worker without unrelated platform setup, and strict result aggregation without duplicate validation jobs.

### Requirement 3: Centralized product and packaging identity

**User story:** As a Copper user, I want consistent installed and downloadable product identity, so that the Rust flavor is distinct from Zed without renaming internal implementation packages.

#### Acceptance criteria

1. **3.1** THE enabled `rust` catalog entry SHALL remain the sole source for Copper display identity, target list, `agentic-tools,rust-tools` application features, `rust-tools` remote-server features, installer template, and artifact template.
2. **3.2** THE generated Rust product metadata SHALL match `products/flavors.toml` and SHALL NOT be hand-edited.
3. **3.3** THE Linux, macOS, and Windows bundlers SHALL consume the resolved product plan while retaining internal `zed`, `cli`, `remote_server`, and updater package names.
4. **3.4** THE final Linux, macOS, and Windows artifact filenames SHALL match the Rust catalog artifact template and supported target architecture.

### Requirement 4: Generated workflow integrity

**User story:** As a repository maintainer, I want workflow YAML to remain generated and reviewable, so that regeneration cannot silently discard pipeline fixes.

#### Acceptance criteria

1. **4.1** THE active workflow behavior SHALL be defined under `tooling/xtask/src/tasks/workflows/` and materialized by `cargo xtask workflows`.
2. **4.2** THE active generated workflow set SHALL include `run_tests.yml`, `release.yml`, and `hosted_collab_tests.yml`, with archived workflow generation policy preserved.
3. **4.3** WHEN generated YAML differs from its generator, THEN repository workflow validation SHALL fail.
4. **4.4** THE workflow generator tests SHALL assert triggers, permissions, runner selection, strict fan-in, catalog features, platform targets, artifact paths, and forbidden private infrastructure.

### Requirement 5: Evidence-based remote integration

**User story:** As a maintainer, I want remote pipeline work dispositioned by results rather than branch names, so that incomplete fixes and unrelated changes are not imported.

#### Acceptance criteria

1. **5.1** FOR EVERY related `origin/codex/*` branch, THE audit SHALL record its tip, unique commits, changed files, intended behavior, overlap, dependencies, validation evidence, and disposition.
2. **5.2** WHEN multiple commits or branches produce the same tree, THEN THE audit SHALL identify the superseding form instead of integrating duplicate histories.
3. **5.3** IF a remote change passes ordinary CI but its release job still fails, THEN THE integration SHALL either correct the demonstrated failure semantically or reject the change with the observed reason.
4. **5.4** THE integration SHALL preserve unrelated Rustlings, Comfy, multiplayer, and upstream Zed behavior and SHALL leave unrelated local work untouched.
5. **5.5** AFTER the reconciled PR is validated and merged, THE cleanup SHALL delete only semantically integrated, superseded, or obsolete remote `codex/*` branches with a recorded full tip SHA, associated PR, rationale, and exact-SHA safeguard; branches with valuable unmerged work or active relevant PRs SHALL remain.

## Constraints

- GitHub Actions references remain pinned where the repository provides pinned helpers.
- Product signing remains optional and secret values remain external.
- The diff remains focused on workflow generation, release packaging hardening, synchronized specs, and generated artifacts.
