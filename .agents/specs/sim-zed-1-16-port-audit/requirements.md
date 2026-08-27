# Requirements: Sim Zed 1.16 port audit

## Problem

The Sim source tree was transplanted from a repository history that represents a
Zed v1.10.2-era tree onto upstream Zed v1.16.1. The rebased tree does not load as
a Cargo workspace, and the unrelated source histories prevent ancestry alone
from proving that Sim behavior survived the transplant.

## Scope

- In scope: preserve the current rebase, identify exact comparison refs, inventory
  the old and rebased tree deltas, reconcile existing Zed dependencies to the
  v1.16.1 declarations, repair port-induced build failures, and collect validation
  evidence for the runnable desktop target and Sim-specific surfaces.
- Out of scope: rewriting the rebase, redesigning Sim features, completing pending
  feature work already documented by imported specifications, publishing, or
  changing external systems.

## Requirements

### Requirement 1: Reproducible port inventory

The maintainer needs a durable mapping between the old Sim tree and its rebased
counterpart so that omissions and adaptations are visible instead of inferred
from a successful build.

#### Acceptance criteria

1. THE audit SHALL record the immutable old base, old tip, new base, and rebased tip commit identifiers used for comparison.
2. IF the old base and old tip have unrelated histories THEN THE audit SHALL report that limitation and compare their tree deltas without claiming an ancestry-based range mapping.
3. THE audit SHALL classify every path changed by the old Sim tree delta as preserved exactly, ported with adaptation, deleted intentionally, or missing/unresolved.
4. THE audit SHALL identify rebased-only paths separately so upstream additions and port adaptations are not mistaken for original Sim behavior.

### Requirement 2: Loadable and buildable workspace

The maintainer needs the rebased workspace to retain Sim feature wiring while
remaining compatible with the newer upstream dependency graph.

#### Acceptance criteria

1. WHEN Cargo loads the rebased workspace THEN every feature dependency SHALL name a declared dependency.
2. WHEN a Sim feature requires a dependency absent from upstream v1.16.1 THEN THE port SHALL retain that dependency using the current workspace dependency convention.
3. WHEN the runnable target is checked THEN port-induced compiler errors SHALL be corrected without discarding newer upstream behavior.

### Requirement 3: Evidence-based functional handoff

The maintainer needs validation beyond compilation to judge the remaining port
risk.

#### Acceptance criteria

1. THE port SHALL pass formatting and the smallest relevant Cargo checks and tests that can run in the local environment.
2. WHEN the desktop launch command can execute locally THEN it SHALL reach application startup without a port-induced build or immediate startup failure.
3. IF a Sim workflow cannot be exercised automatically or in the local environment THEN the handoff SHALL name that workflow and the missing verification condition.
4. THE handoff SHALL distinguish exact preservation, semantic adaptation, successful automated validation, and behavior that remains unverified.
5. IF the build host lacks the optional Apple Metal command-line toolchain THEN THE port SHALL preserve the upstream shader-build default and SHALL document the supported runtime-shader launch used for local validation.

### Requirement 4: Upstream-authoritative dependency reconciliation

The maintainer needs existing Zed dependencies to retain the reviewed v1.16.1
source and platform configuration while Sim-only crates retain their genuinely
new requirements.

#### Acceptance criteria

1. FOR every dependency declaration that existed in Zed v1.16.1 THE rebased manifests SHALL preserve the v1.16.1 repository URL, revision, package name, version, features, and platform configuration.
2. THE reconciliation SHALL retain dependencies required only by genuinely new Sim crates or functionality without replacing an existing Zed dependency.
3. IF Sim behavior appears to require a fork of an existing Zed dependency THEN the audit SHALL identify the affected behavior, fork-only code, upstream limitation, and smallest adaptation, and SHALL NOT select the fork without maintainer approval.
4. WHEN manifests are reconciled THEN Cargo.lock SHALL be regenerated from those manifests rather than copied from the old Sim tree or repaired entry-by-entry.
5. THE audit SHALL report source or version drift from the v1.16.1 dependency graph after lockfile regeneration.

### Requirement 5: Complete Sim icon branding (superseded)

The maintainer needs every Sim-branded icon variant to resolve to a bundled SVG
so renamed UI surfaces render without repeated asset-loading errors.

This requirement and its completed evidence describe the initially attempted
rename. Requirement 7 supersedes it: existing Zed icon identities remain
upstream-authoritative and Sim branding will be introduced separately.

#### Acceptance criteria

1. FOR every `IconName` variant THE asset bundle SHALL contain the SVG path produced by that variant.
2. WHEN a Zed-branded icon variant is renamed for Sim THEN the corresponding asset SHALL use the same Sim-branded file stem without changing its artwork unintentionally.
3. THE asset bundle SHALL contain no dangling top-level icon SVGs that cannot be parsed as an `IconName`.

### Requirement 6: Complete Sim vector-image branding (superseded)

The maintainer needs every Sim-branded vector image to resolve to a bundled SVG
so the welcome and onboarding surfaces render without asset-loading errors.

This requirement and its completed evidence describe the initially attempted
rename. Requirement 7 supersedes it for existing upstream vector images.

#### Acceptance criteria

1. FOR every `VectorName` variant THE asset bundle SHALL contain the SVG path produced by that variant.
2. WHEN a Zed-branded vector variant is renamed for Sim THEN the corresponding asset SHALL use the same Sim-branded file stem without changing its artwork unintentionally.
3. THE asset bundle SHALL contain no dangling top-level image SVGs that cannot be parsed as a `VectorName`.

### Requirement 7: Upstream-authoritative asset reconciliation

The maintainer needs existing Zed assets and their code identities to remain
exactly aligned with v1.16.1 while genuinely new Sim assets remain separate.

#### Acceptance criteria

1. FOR every file present under `assets/` in v1.16.1 THE current tree SHALL preserve its path and byte content exactly.
2. FOR existing upstream icon and vector identities THE current tree SHALL preserve the v1.16.1 enum variant and update every Rust call site to that identity.
3. THE reconciliation SHALL retain genuinely new Sim asset files separately and SHALL report them without treating them as replacements for upstream assets.
4. THE exhaustive icon and vector tests SHALL validate the restored upstream names and paths in both directions.
5. WHEN the stateless application starts THEN the asset loader SHALL emit no missing-asset errors caused by this reconciliation.
