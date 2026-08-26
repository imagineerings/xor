# Requirements: Declarative multi-product flavors

## Problem

Product identity and language focus are currently distributed across Rust constants, Cargo bundle metadata, platform scripts, menus, URL parsing, single-instance code, updater requests, and generated workflows. The current fork also contains direct Rustlings branding in stable packaging while runtime paths and protocol identities remain Zed-oriented. This makes additional branded products likely to collide with Zed or each other and turns every rename into a risky repository-wide edit.

Maintainers need one declarative product catalog that can produce independently installable, language-focused applications while preserving the shared Zed implementation and minimizing upstream divergence.

## Scope

### In scope

- Define stable internal flavors `rust`, `jvm`, and `game`, with provisional marketing names kept separate from those IDs.
- Make product identity, capabilities, onboarding defaults, packaging, release artifacts, and update namespaces declarative.
- Introduce one product-aware bundling command for Linux, macOS, and Windows.
- Isolate installed products from Zed and from one another at every operating-system identity and user-data boundary.
- Simplify generated CI around shared validation, product smoke tests, and a product/platform release matrix.
- Implement only the Rust flavor in Phase 1; retain JVM and Game as disabled, planned catalog entries until later phases.

### Out of scope

- Renaming internal `zed` crates, modules, actions, project-local `.zed` configuration, or upstream protocol types solely for branding.
- Removing general-purpose language support from any flavor; a flavor changes defaults, not the editor's ability to open other languages.
- Implementing JVM- or Game-specific product builds in Phase 1.
- Choosing permanent marketing names, publisher domains, production signing credentials, or a production update-hosting provider in this specification.
- Bulk replacement of `Zed` or `zed` strings across the repository.

## Glossary

- **Flavor ID**: Stable, non-marketing identifier used by build and release automation. The initial IDs are `rust`, `jvm`, and `game`.
- **Marketing name**: User-facing product name that may change without changing the flavor ID.
- **Product catalog**: The versioned declarative manifest containing product-specific identity and default configuration.
- **Release channel**: A lifecycle stream such as stable, preview, nightly, or development within one product.
- **Product namespace**: A stable product-owned value used to derive data paths, process coordination identities, and update channels.

## Requirements

### Requirement 1: Declarative product catalog

**User story:** As a maintainer, I want one validated catalog for all product-specific choices, so that a new flavor or marketing rename does not require rediscovering scattered branding logic.

#### Acceptance criteria

1. **1.1** THE product catalog SHALL define the stable flavor IDs `rust`, `jvm`, and `game` independently from their provisional marketing names.
2. **1.2** FOR EACH catalog entry, THE catalog SHALL define its display name, executable name, bundle identifier, URL scheme, icon set, application data/config namespace, Cargo features, default extensions, default language servers, toolchain onboarding, agent profile and default-instruction asset, installer-name template, release-artifact-name template, and update namespace.
3. **1.3** WHEN the catalog contains a missing required field, unknown status, malformed template, unsupported feature mapping, unsafe path, or duplicate operating-system identity, THEN THE catalog validator SHALL fail before a bundle or workflow is generated.
4. **1.4** WHEN a marketing name changes, THEN THE flavor ID and technical namespaces SHALL remain unchanged unless a separate, explicit identity migration changes them.
5. **1.5** THE selected product configuration embedded in a release build SHALL be compile-time immutable and inspectable through a dry-run or metadata command.

### Requirement 2: Product identity and coexistence

**User story:** As a developer using multiple IDE products, I want them to coexist with Zed and with one another, so that launching or configuring one product cannot affect another.

#### Acceptance criteria

1. **2.1** WHEN a product is bundled, THEN its installed display name, executable or launcher name, application identifier, URL handler, icons, menu/application labels, and About/version presentation SHALL reflect the selected catalog entry.
2. **2.2** FOR EVERY product and release-channel pair, THE derived bundle/application ID, global data/config/cache/state/log namespace, remote-server installation namespace, mutex or lock identity, IPC endpoint, URL scheme registration, installer identity, and updater channel SHALL be distinct from Zed and every other product/channel pair.
3. **2.3** THE implementation SHALL retain internal `zed` crate, module, action, and project-local `.zed` names wherever they are not operating-system or user-facing product identities.
4. **2.4** THE implementation SHALL apply branding through typed configuration consumers and exact metadata fields, and SHALL NOT perform global string replacement over source files or built bundles.
5. **2.5** THE project-local `.zed` directory and compatible project settings SHALL remain shared so that a project can be opened by Zed or any flavor without duplicating project configuration.

### Requirement 3: Language-focused defaults

**User story:** As a user of a language-focused product, I want useful tools and guidance on first launch while retaining control over my editor configuration.

#### Acceptance criteria

1. **3.1** WHEN the Phase 1 Rust flavor is built, THEN the `zed` package SHALL be built with exactly the catalog-selected `multiplayer-tools` and `rust-tools` features, agentic support SHALL remain enabled transitively through `multiplayer-tools`, and its remote server SHALL receive only the corresponding Rust tooling feature.
2. **3.2** WHEN a product starts with a fresh product namespace, THEN it SHALL apply the catalog's product-default settings, extensions, and language-server ordering below user and project overrides.
3. **3.3** IF a user removes, disables, or overrides a product default, THEN a later launch or catalog revision SHALL NOT silently restore that default; failed default-extension installation SHALL be visible and retryable.
4. **3.4** WHEN toolchain onboarding runs, THEN it SHALL report detected and missing product toolchains and offer explicit guidance or user-approved actions without silently installing or modifying system toolchains.
5. **3.5** WHEN the agentic feature is present, THEN the selected product's default agent profile and instruction asset SHALL be available as defaults while personal `AGENTS.md` and project rules retain higher precedence.
6. **3.6** THE `jvm` and `game` entries SHALL remain non-buildable and excluded from generated smoke/release matrices until their future enablement criteria pass.

### Requirement 4: One product-aware bundling interface

**User story:** As a release engineer, I want one command to configure platform packaging, so that scripts cannot drift onto different names or feature sets.

#### Acceptance criteria

1. **4.1** WHEN `cargo xtask bundle --product <id>` is invoked for a supported host/target, THEN THE command SHALL validate the product, resolve its build plan, and invoke the existing platform bundler with all product settings supplied from that plan.
2. **4.2** THE build plan SHALL use an isolated product/target output directory and SHALL pass explicit `--no-default-features` and catalog feature lists to application and remote-server builds.
3. **4.3** WHEN bundling succeeds, THEN the installed application, installer, and final release artifact SHALL use the catalog's exact product-specific names without relying on post-build global replacement.
4. **4.4** IF the flavor ID is unknown, planned, incompatible with the requested target, or has an invalid build plan, THEN THE bundle command SHALL fail before compiling.
5. **4.5** IF signing credentials are absent, THEN the default signing policy SHALL produce an unsigned or ad-hoc development bundle; IF signing is explicitly required, THEN missing or incomplete credentials SHALL fail before packaging.
6. **4.6** THE catalog and build plan SHALL contain no credential values, and signing credentials SHALL enter only through the existing platform secret environment at execution time.

### Requirement 5: Shared CI and product releases

**User story:** As a maintainer, I want CI effort shared across the common codebase and releases expanded from product data, so that adding a flavor does not clone entire workflows.

#### Acceptance criteria

1. **5.1** WHEN CI runs, THEN formatting, linting, and the appropriate shared workspace tests SHALL execute once rather than once per product.
2. **5.2** AFTER shared validation succeeds, THE generated CI SHALL run a focused build or bundle-plan smoke test for every enabled product and SHALL verify its explicit application and remote-server features.
3. **5.3** WHEN a product release is requested, THEN THE generated release workflow SHALL expand enabled products and supported platforms through a product/platform matrix and SHALL upload separately named product artifacts.
4. **5.4** THE publishing stage SHALL run only after all selected matrix builds succeed and SHALL publish artifacts and update metadata into the selected product's update namespace without mixing another product's assets.
5. **5.5** THE workflow generator SHALL derive enabled products, feature smoke commands, artifact names, and release matrix entries from the validated catalog, while keeping action permissions minimal and signing inputs optional.

### Requirement 6: Phased migration and extension

**User story:** As a fork maintainer, I want to replace ad hoc Rust branding safely before adding more products, so that current users and upstream merges are not put at risk.

#### Acceptance criteria

1. **6.1** PHASE 1 SHALL enable and release only the `rust` flavor, using the provisional marketing name **Copper**; **Orbit** for `jvm` and **Forge** for `game` SHALL remain planning names on disabled entries.
2. **6.2** WHEN Phase 1 is complete, THEN hard-coded Rustlings stable-package values SHALL be removed from product consumers and represented by the `rust` catalog entry or typed derivations.
3. **6.3** THE migration SHALL NOT automatically import from existing Zed-named data/config paths because the current fork and upstream Zed cannot be distinguished safely there.
4. **6.4** WHERE legacy settings import is offered, THE user SHALL select or confirm the source, the import SHALL copy rather than move data, and a failure SHALL leave both the source and the new product namespace usable.
5. **6.5** WHEN a future JVM or Game phase is implemented, THEN enabling its existing flavor ID SHALL reuse the same runtime, bundling, updater, and workflow boundaries without introducing flavor-specific branding branches across unrelated crates.

## Constraints

- Permanent publisher-owned bundle identifiers and update-feed locations require an explicit decision before production release.
- Provisional marketing names and icon artwork may change; stable flavor IDs and technical namespaces must not derive from those names.
- Product-default configuration must remain lower precedence than user settings, personal agent instructions, project settings, and project rules.
- Build and workflow generation must remain deterministic and reviewable from repository contents.
- Optional signing must reuse platform mechanisms already supported by the repository; no credentials or fabricated secrets may be committed.

## Open questions

- What publisher-owned reverse-DNS root should replace the provisional `dev.ideflavors` bundle-identifier root before production signing?
- Which service will host the per-product update manifests consumed by the existing updater contract?
- Should a future migration tool support selective import of themes, keymaps, and extensions, or only whole explicitly selected config/data directories?
