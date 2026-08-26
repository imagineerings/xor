# ADR: Native profiling collector and artifact gate

- Status: Proposed no-go
- Date: 2026-08-26
- Owners: Rust tools platform and profiling
- Scope: Native profile model/view only; explicit external Tasks remain supported

## Decision

Do not implement a native profile result model or view in this pack.

Samply 0.13.1 is a credible future collector candidate because its upstream project supports macOS, Linux and Windows and is dual Apache-2.0/MIT licensed. It can save a Firefox Profiler JSON artifact instead of opening a browser. That is enough to preserve an external Task recipe, but it does not yet satisfy the native-product evidence gate: the consumed JSON contract is not published as a stable, versioned ingestion schema; the profile carries host paths and symbol/source facts that require a reviewed redaction contract; and no bounded remote-transfer or cancellation conformance fixture has been accepted.

Tasks `rust-profiling/3.2` and `rust-profiling/3.3` therefore remain deferred. This is a no-go decision, not approval to introduce product types.

## Evidence

| Concern | Evidence | Disposition |
| --- | --- | --- |
| Collector and version | Samply 0.13.1 is the latest upstream release listed as of this review. Its upstream README describes macOS, Linux and Windows collection. | Candidate only; Zed does not install or bundle it. |
| License/distribution | Upstream declares Apache-2.0 OR MIT and publishes per-platform archives. | License is compatible with evaluation, but distribution is out of scope. |
| Platform behavior | Linux uses perf events and may require permission changes; macOS attach requires code signing; Windows 0.13.1 uses ETW/xperf and administrator privileges. | Support cannot be represented as one unconditional platform promise. |
| Artifact | `samply record --save-only -o profile.json.gz ...` produces Firefox Profiler JSON on the three desktop platforms. | External Tasks may declare the resulting bounded artifact. |
| Format stability | Firefox Profiler maintains Flow/TypeScript profile types, while its format documentation distinguishes source and processed formats and describes ongoing migration. | No stable versioned subset has been selected for a native parser. |
| Privacy | Profile structures include absolute library/debug paths, process arguments when requested, symbols and optional source data. Upstream documentation also warns that profiles can contain sensitive information. | Raw remote transfer is rejected until field-level filtering and consent are specified. |
| Network | Samply's default workflow opens profiler.firefox.com and can use symbol servers. | Any external recipe must use save-only, must not configure symbol servers implicitly and remains user-authored. |
| Cancellation | Zed Tasks already own process cancellation and kill-on-drop behavior. | Native parsing still needs a deterministic late-artifact rejection fixture before approval. |

Primary sources:

- [Samply repository, platform behavior and license](https://github.com/mstange/samply)
- [Samply 0.13.1 release and Windows/privilege notes](https://github.com/mstange/samply/releases/tag/samply-v0.13.1)
- [Firefox Profiler profile types](https://github.com/firefox-devtools/profiler/blob/main/src/types/profile.ts)
- [Firefox Profiler format documentation](https://github.com/firefox-devtools/profiler/blob/main/docs-developer/gecko-profile-format.md)
- [Firefox profiling privacy warning](https://firefox-source-docs.mozilla.org/tools/profiler/markers-guide.html)

## External Task contract retained now

The supported convenience is deliberately smaller than a collector integration:

1. The user supplies the profiler executable and structured arguments.
2. The command runs through existing Tasks on the authoritative project host after normal trust and explicit-invocation checks.
3. The task may declare one project-relative SVG, HTML or other supported file artifact.
4. Zed opens the artifact only after successful completion, only when it is visible in the project, and only within the declared byte limit.
5. Missing, oversized, private or disconnected artifacts remain explicit failures. Zed never discovers an artifact by parsing terminal output.

## Approval checklist for reconsideration

A later ADR may change this decision only after all of the following evidence is attached and reviewed:

- pin a collector/version policy and document supported OS/architecture/privilege cells;
- select a versioned machine-readable schema or a strict stable subset with malformed/forward-version behavior;
- define frame, sample, edge, string, file and compressed/decompressed byte caps;
- define absolute-path, arguments, environment, symbol, source and library redaction rules;
- prove save-only/no-network collection and reject implicit symbol-server traffic;
- define remote consent, transfer chunking, cancellation and late generation rejection;
- add deterministic macOS and one non-macOS capture fixtures with equivalent semantics;
- complete licensing, accessibility and product approval reviews.

Until then, the required fallback is the external Task/artifact flow and external viewers.
