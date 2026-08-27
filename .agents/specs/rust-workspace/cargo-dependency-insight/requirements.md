# Requirements: Cargo dependency insight

## Purpose and status

This Next pack owns richer read-only dependency and feature provenance beyond the Cargo dashboard's existing direct declaration annotations. No richer provenance UI/model was found. The dashboard remains direct-only; this pack must not turn it into an unbounded recursive graph.

Canonical IDs are `cargo-dependency-insight/<criterion>`.

### Requirement 1: Explain bounded Cargo dependency provenance [Deferred / Next]

#### Acceptance criteria

1. **1.1** WHEN a direct dependency is selected, THE system SHALL distinguish its visible declaration, rename, normal/dev/build kind, optional/default/requested features, target condition, path/registry/Git source, workspace inheritance when safely derivable, resolved package/version/source, and lock status when available.
2. **1.2** THE first UI SHALL show a package-centric detail projection for the selected direct dependency and SHALL NOT recursively expand a transitive dependency tree or create cycles.
3. **1.3** WHEN feature activation provenance is exposed by validated Cargo data, THE view SHALL distinguish package-defined features, explicitly requested dependency features and resolved enabled features without claiming a unique cause Cargo did not expose.
4. **1.4** Activating a declaration/resolved local package SHALL navigate only to a safely visible manifest; external registry/Git sources SHALL show provenance without opening host-private cache paths.
5. **1.5** Missing lockfiles, incomplete metadata, unknown source kinds, multiple resolved versions, stale snapshots and malformed manifests SHALL be represented explicitly and SHALL preserve valid facts.
6. **1.6** Models/protocols SHALL be deterministic, generation-bound and bounded by packages/edges/features/strings; cycles and truncation SHALL be observable.
7. **1.7** Discovery SHALL run with existing Cargo metadata/configuration collection on the authoritative host, obey trust/remote/multiplayer visibility, perform no client-local fallback, and initiate no fetch/network command.
8. **1.8** Tests SHALL cover renamed/optional/target-specific/path/registry/Git/workspace-inherited declarations, multiple versions, cycles, missing/stale locks, remote filtering, malformed data and a large synthetic graph.

## External, rejected and non-goal boundaries

`cargo audit`, `cargo deny`, `cargo outdated`, unused-dependency analysis, license/vulnerability databases and registry freshness are External explicit Tasks, not this feature. Dependency addition/removal/update, feature mutation, automatic network, graphical graphs and a universal package model are rejected/out of scope.

## Open questions

1. **First-level detail depth.** Recommended default: one selected direct declaration with its resolved instance(s), immediate feature facts and optional workspace-member navigation; no recursively expandable children. UI/model tasks depend on this cap.
