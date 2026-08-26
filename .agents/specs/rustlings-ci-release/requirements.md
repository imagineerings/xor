# Requirements: Rustlings CI and release

## Problem

The fork inherits large, organization-specific Zed workflows that require private runners, caches, services, credentials, and release coordination. Fork maintainers need a small GitHub-hosted pipeline that validates Rustlings and publishes installable builds with the agentic and Rust tooling capabilities enabled.

## Scope

### In scope

- Replace the two active generated workflows with focused Rustlings CI and release workflows.
- Build Linux x86_64, macOS ARM64, and Windows x86_64 release artifacts with `multiplayer-tools` and `rust-tools` enabled; `multiplayer-tools` enables agentic functionality transitively.
- Give the installed stable application and downloadable artifacts Rustlings branding.
- Support unsigned builds when platform signing credentials are absent.

### Out of scope

- Renaming internal crates, modules, executable protocol names, or every user-facing Zed string.
- Publishing a live GitHub release during local implementation.
- Preserving organization-only compliance, collaboration, extension, documentation, notification, or cache jobs in the active workflows.

## Requirements

### Requirement 1: Focused continuous integration

**User story:** As a fork maintainer, I want one understandable validation job, so that pull requests and main-branch changes receive useful Rust feedback without private infrastructure.

#### Acceptance criteria

1. **1.1** WHEN a pull request, a push to `main`, or a manual dispatch occurs, THEN THE CI workflow SHALL run one Linux validation job on a standard GitHub-hosted runner.
2. **1.2** THE validation job SHALL check formatting, run `./script/clippy`, execute workspace tests with cargo-nextest, and check a release build of `zed` with exactly `multiplayer-tools` and `rust-tools` enabled.
3. **1.3** WHEN a newer run starts for the same workflow and ref, THEN THE workflow SHALL cancel the superseded run.
4. **1.4** THE active CI workflow SHALL NOT require an organization-owner guard, private runner, R2 cache, Sentry, Slack, PostgreSQL service, or unrelated Zed automation.

### Requirement 2: Minimal cross-platform release

**User story:** As a release maintainer, I want tag-driven cross-platform builds, so that a single Rustlings GitHub Release contains the supported installers.

#### Acceptance criteria

1. **2.1** WHEN a `v*` tag or manual dispatch starts the release workflow, THEN THE workflow SHALL build Linux x86_64, macOS ARM64, and Windows x86_64 bundles on matching standard GitHub-hosted runners.
2. **2.2** THE platform jobs SHALL reuse the existing bundle scripts and enable exactly `multiplayer-tools` and `rust-tools`; agentic functionality SHALL remain enabled transitively through `multiplayer-tools`.
3. **2.3** WHEN every platform build succeeds, THEN THE publishing job SHALL create or update a GitHub Release titled `Rustlings <tag>` and attach all Rustlings artifacts.
4. **2.4** IF any platform build fails, THEN THE publishing job SHALL NOT publish a release.
5. **2.5** THE release workflow SHALL use read-only contents permission by default and grant contents write permission only to the publishing job.
6. **2.6** THE active release workflow SHALL NOT contain a hard-coded external repository target or require organization-specific runners, compliance checks, notifications, caches, or service credentials.

### Requirement 3: Rustlings packaging identity

**User story:** As a Rustlings user, I want downloads and installed applications to use the Rustlings name, so that the fork is distinguishable from Zed.

#### Acceptance criteria

1. **3.1** THE stable Linux desktop entry, macOS bundle, and Windows installer SHALL display the name `Rustlings`.
2. **3.2** THE published Linux, macOS, and Windows artifact filenames SHALL begin with `rustlings-`.
3. **3.3** THE implementation SHALL retain internal `zed` crate and executable names where they are required by existing runtime and packaging code.
4. **3.4** IF platform signing credentials are absent, THEN THE bundle scripts SHALL produce unsigned or ad-hoc-signed development artifacts instead of failing because credentials are missing.

### Requirement 4: Generated workflow integrity

**User story:** As a repository maintainer, I want workflow definitions to remain generator-owned, so that future regeneration does not discard the simplification.

#### Acceptance criteria

1. **4.1** THE workflow behavior SHALL be defined under `tooling/xtask/src/tasks/workflows/` and materialized by `cargo xtask workflows`.
2. **4.2** THE generated active workflow set SHALL remain `release.yml` and `run_tests.yml` and SHALL pass the repository workflow validation.

## Constraints

- GitHub Actions references should remain pinned to commit SHAs where the repository already provides pinned helpers.
- Local delivery must not create or publish an external GitHub Release.
- The diff should avoid broad product rebranding beyond the stable packaging surfaces needed by this release pipeline.
