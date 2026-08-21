# Requirements: Goose Documentation Coverage in Zed

## Introduction

Port relevant Goose documentation behavior and content into Zed's existing mdBook site and release pipeline. Docusaurus, React themes/plugins, package management, and Goose branding are upstream infrastructure, not parity requirements. Zed already builds, link-checks, versions, and deploys `docs/`; this migration extends that system only where approved Goose capabilities need documentation.

## Requirements

### Requirement 1: Capability Documentation

**User Story:** As a Zed user, I want accurate migration-feature documentation, so that I can install, configure, use, and troubleshoot approved Goose-derived behavior.

#### Acceptance Criteria

1. **1.1** EACH approved user-facing migration capability SHALL have installation/setup, configuration, usage, failure behavior, security/privacy, platform, and compatibility documentation where applicable
2. **1.2** DOCUMENTATION SHALL name the Zed-native component and terminology and SHALL NOT claim an upstream command, provider, service, or UI exists until implementation evidence and tests support it
3. **1.3** PROVIDER, extension, recipe, authentication, scheduling, gateway, local-model, and embedded-app documentation SHALL disclose prerequisites, permissions, credential handling, feature gates, unsupported platforms, and safe recovery
4. **1.4** EXAMPLES SHALL be runnable against approved public surfaces and SHALL declare prerequisites, expected output, failure cases, and maintenance owner

### Requirement 2: Existing Zed Documentation Architecture

**User Story:** As a maintainer, I want migration docs to use Zed's established site, so that navigation, release channels, links, and deployment remain consistent.

#### Acceptance Criteria

1. **2.1** THE migration SHALL extend `docs/src`, `docs/SUMMARY.md`, existing preprocessors, and existing style/voice rules instead of introducing Docusaurus
2. **2.2** NEW pages SHALL integrate with the current navigation hierarchy and stable/preview/nightly release-channel policy
3. **2.3** INTERNAL and generated links, action references, examples, and assets SHALL pass the existing mdBook, preprocessor, and link checks
4. **2.4** SEARCH and hosting SHALL continue through Zed's current documentation deployment; no separate Goose search index or site deployment SHALL be created

### Requirement 3: Source Maps and Machine-Consumable Artifacts

**User Story:** As a developer or agent, I want trustworthy documentation indexes, so that approved capabilities can be discovered without stale generated data.

#### Acceptance Criteria

1. **3.1** WHERE Goose docs maps, `llms.txt`, server catalogs, or skills manifests provide required observable discoverability, THE equivalent SHALL be generated from Zed's canonical sources
2. **3.2** GENERATED artifacts SHALL be deterministic, validated in CI, and SHALL fail generation on invalid source metadata rather than publish stale partial output
3. **3.3** SECRETS, private endpoints, unpublished features, and local paths SHALL NOT appear in generated public artifacts

### Requirement 4: Tutorials, Blog, and Community Content Boundary

**User Story:** As a product owner, I want content types added intentionally, so that a migration does not create unsupported publishing obligations.

#### Acceptance Criteria

1. **4.1** APPROVED workflow tutorials SHALL use Zed's existing documentation patterns and SHALL be validated against the corresponding implementation or test fixture
2. **4.2** GOOSE blog posts, author/tag metadata, community-star automation, and marketing assets SHALL be excluded unless a separate content/marketing decision approves a Zed-native equivalent
3. **4.3** UPSTREAM legal, branding, analytics, theme, and deployment configuration SHALL NOT be copied as product documentation behavior

### Requirement 5: Documentation Validation and Release

**User Story:** As a maintainer, I want documentation changes validated by existing automation, so that migration pages do not break the site or advertise unavailable work.

#### Acceptance Criteria

1. **5.1** DOCUMENTATION tasks SHALL run the existing formatter, mdBook build, internal/external link checks appropriate to CI, and affected generated-artifact checks
2. **5.2** RELEASE-channel behavior SHALL reuse `.github/workflows/deploy_docs.yml` and associated xtask generation rather than add a parallel workflow
3. **5.3** A migration documentation review SHALL trace each capability claim to its requirement, implementation evidence, and verification status before publication

## References

- Goose: `projects/goose/documentation/docusaurus.config.ts`, `sidebars.ts`, `docs/`, `blog/`, `scripts/`, `plugins/`, `static/`
- Zed: `docs/book.toml`, `docs/SUMMARY.md`, `docs/src/`, `docs/.rules`
- Zed: `crates/docs_preprocessor/`, `script/check-links`, `script/prettier`, `script/docs-suggest*`
- Zed: `.github/workflows/deploy_docs.yml`, `.github/workflows/run_tests.yml`
