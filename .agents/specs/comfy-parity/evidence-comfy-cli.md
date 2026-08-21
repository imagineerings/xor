# Comfy CLI parity evidence

## Audit status

This report records the static and existing-test evidence gathered from `projects/comfy/comfy-cli` for the native Rust/GPUI parity design. Comfy CLI is evidence and a development-time conformance client; it does not authorize a production Python dependency. Production Zed must implement execution and lifecycle natively and may accept legacy Python-oriented formats only for migration or compatibility translation into versioned Rust/WASM plugins with explicit ports.

No nested `AGENTS.md` or nested Git metadata exists in this source root. README, design, skill, and guide claims are never promoted above `documented-only` without executable or test corroboration.

## Source baseline

| Property | Evidence-backed value |
|---|---|
| Source root | `projects/comfy/comfy-cli` |
| Git identity | No nested `.git`; no commit SHA asserted. |
| Manifest version | `0.0.0`, explicitly a CI release-time placeholder, not a release version. |
| Required Python | `>=3.10` |
| Source-tree files | 312 |
| Deterministic fingerprint | SHA-256 `09d0b5f262bce3105f83777a310f1e391c4624f95142da5e3230626b68a276e6` |
| Fingerprint recipe | Sort included relative paths bytewise, hash each file, then hash lines of `<digest>  ./<relative-path>`. Excludes `.git`, `node_modules`, `__pycache__`, `*.pyc`, and `.DS_Store`. |

The 312-file closure is: 137 packaged runtime files, 141 tests/fixtures, 5 `docs/` files, 14 `.github/` files, 2 demonstration assets, and 13 root packaging/metadata files. Packaged runtime contents are 104 Python modules, 24 JSON files, 6 Markdown resources, 2 YAML registries, and one OpenAPI YAML document.

## Runtime constraints

The available interpreter is Python 3.9.6, below the declared minimum. Typer, questionary, PyYAML, pytest, and other dependencies are absent. `PYTHONPATH=projects/comfy/comfy-cli python3 -m comfy_cli --help` reaches the entry point but fails on `ModuleNotFoundError: questionary`. No dependency or network mutation was authorized, so no command behavior is labelled observed and no existing test was run. Static syntax validation parsed all 228 Python files (104 production plus 124 tests) without error. Thirty of 31 `.json` files parse as strict JSON; `pyrightconfig.json` is JSON-with-a-trailing-comma configuration, not a runtime schema.

## Registry reconciliation

| Surface | Source | Catalog | Result |
|---|---:|---:|---|
| Reachable leaf command paths | 123 | 123 | Match; 41 top-level names. |
| Typer app objects | 20 | 20 | Match. |
| `@command` registrations / unique functions | 113 / 112 | represented by path | Duplicate is stacked `dependency` decorators. |
| Root/global options | 11 | 11 | Match. |
| Command-path parameter bindings, including aliases and fixed `generate` grammar | 370 | 370 | Match with zero unresolved rows; every row has typed arity/cardinality evidence, while 55 retain non-empty explicit parser constraints, 6 retain exact Enum choices, and 15 retain paired boolean spellings. Schema-derived per-partner fields live in the 52 endpoint schemas and are not falsely counted as Typer flags. |
| JSON schemas | 23 | 23 | Match. |
| Command-schema mappings | 64 | 64 | 63 reachable plus orphan `comfy version`. |
| Stream-schema mappings | 2 | 2 | Match. |
| Error codes | 99 | 99 | Unique and bidirectionally ratcheted by tests. |
| Versioned event union | 12 | 12 | Four code/schema mismatches are explicit. |
| Production environment variables | 35 | 35 | Match. |
| Persisted config keys | 20 | 20 | Match. |
| Persisted/interchange formats | 34 | 34 | Match. |
| Lifecycle contracts | 24 | 24 | Match. |
| Extension contracts | 17 | 17 | Match. |
| Partner allowlist / aliases | 52 / 52 | 52 | Every allowlisted path exists in the vendored OpenAPI. |
| OpenAPI paths / operations / excluded operations / proxy paths | 268 / 289 / 234 / 193 | reconciled metadata | Match. |
| CQL labels / packs / node-label entries / assignments | 10 / 87 / 322 / 432 | 419 policy rows | Match; 83 packs declare a version field, 48 contain a non-empty registry pin, and 38 pack names contain a git-ref pin. |
| Test functions / classes / fixtures | 2,295 / 316 / 129 | 2295 function rows | Functions match; tests inspected, not run. |
| Production Python modules | 104 | 104 | Match. |
| Source files | 312 | 312 | Every production file maps to stable IDs; every other file has an explicit disposition. |

## Capability disposition

The behavioral capability catalogs contain 1,244 stable records, alongside 104 production module/service contracts, 2,295 test-function records, 312 source-file rows, and 66 schema-mapping relationships. Their evidence split is 288 test-backed, 947 code-inferred, 9 documented-only, 0 observed, and 0 unverified. The master ledger promotes both behavioral and production module/service records so every production source row closes against a master feature ID. Test-backed means an existing test explicitly exercises the contract; it does not imply that the test ran in this audit.

Source-audit native-target dispositions are 591 missing, 554 conflicting, 99 deferred, 0 equivalent, 0 partial, and 0 uncertain. The master generator synchronizes target-only columns against independent Zed evidence and the fixed native-only architecture before producing the pack-wide parity matrix.

## Command and machine-contract findings

The reachable tree contains 123 leaves and 41 top-level names. It includes local execution/jobs, workflow conversion/editing/fragments, node and model introspection, project assets, templates, previews, custom-node/Manager compatibility, agent skills, cloud OAuth/jobs/workflows, partner generation, and hidden/developer aliases.

Three source orphans are retained rather than normalized away:

- `comfy models (legacy hidden function)` — Shadowed by the visible `models` Typer group; retain as deprecated/dead source evidence.
- `comfy version` — Advertised by COMMAND_SCHEMAS but no command registration exists; global --version is the executable surface.
- `comfy query` — Only HELP_EXAMPLES/run-cli text mention it; no command registration exists.


`COMMAND_SCHEMAS` contains 64 entries, but only 63 target reachable paths; `comfy version` is not registered. Sixty reachable leaves have no command-schema mapping. This is not interpreted as absence of behavior: many legacy/interactive commands simply have not migrated to the structured envelope registry.

The event contract has a concrete versioning conflict. `run_event.json` declares eight event names. Executable code additionally emits `converted`, `prompt_preview`, `settled`, and `state`; the first two are also described in `docs/json-output.md`. Native Zed must define one authoritative event union and validate every emitted line against it.

## Typed parameter contracts

The parameter ledger retains 355 distinct bindings plus 15 alias-path repetitions. Of its 370 rows, 360 row bindings derive from statically parsed Python annotations and 10 derive from the explicit `generate` tail parser branches. Value types are {"boolean": 82, "enum": 6, "integer": 41, "number": 6, "path": 7, "string": 228}; 200 rows are nullable, 22 accept repeated or variadic values, 6 have exact statically resolved Enum choices, 55 retain explicit callback/autocompletion/metavar/input constraints, and 15 expose paired boolean spellings.

This is static contract evidence, not observed Typer behavior. Callback and autocompletion expressions are retained by source name but are not executed; prose-only examples and help-text suggestions are not promoted into enforced choices; only Enum declarations become exact `choices`. The ten dynamic `generate` rows are typed from the parser's explicit boolean, numeric, string, path, and default branches. Schema-derived partner fields remain in the partner endpoint schemas instead of being misrepresented as fixed Typer bindings.

## Native architecture consequences

Command status counts are missing 73, conflicting 40, and deferred 10. `conflicting` commands are Python/ComfyUI-Manager process operations that cannot be copied into production. Their observable intent becomes native installation/update/runtime/plugin behavior, or a legacy import/migration surface. Cloud, partner-generation, telemetry feedback, and code-search mutations remain explicit deferred service contracts rather than disappearing.

The source defines 99 error codes, envelope/1 and event/1 machine protocols, UI/API workflow conversion, object_info schema behavior, queue/history/jobs/cancellation, local/cloud routing, durable job recovery, 34 persisted/interchange formats, and 17 extension contracts. These are high-value conformance inputs for a native Rust core and compatibility server/CLI.

Python custom-node packaging and cm-cli execution are architectural conflicts. Native parity requires:

- a versioned Rust/WASM plugin manifest;
- explicit typed input/output ports and list/lazy/output-node semantics;
- capability permissions for files, network, state, custom routes, and large outputs;
- deterministic resource/memory/cancellation boundaries;
- legacy `class_type`, socket, pack, and registry identifiers mapped to native plugin/version identifiers;
- import diagnostics for unmapped Python nodes without executing their code.

## Tests and source coverage

The test catalog contains 2,295 `test_*` functions in 124 Python files, 316 `Test*` classes, and 129 fixtures. Opt-in E2E suites cover real installation, launch, model operations, custom-node lifecycle, execution, GPU, unified dependency resolution, conflict attribution, and telemetry delivery. They were not run because the environment lacks the declared runtime/dependencies and real E2E paths would clone/download/mutate external state.

Source coverage contains one deterministic row for every vendored file. No production file is unmapped. Documentation, tests, CI, assets, manifests, and locks have explicit non-production dispositions. The machine catalogs preserve documentation-only and dead/orphan claims instead of treating them as executable evidence.

## Generated catalogs

The authoritative files are `catalogs/comfy-cli-*.csv` and `catalogs/comfy-cli-reconciliation.json`, regenerated by `generate_comfy_cli_catalogs.py`. Catalog hashes are recorded in the reconciliation JSON generated after the rows are written.
