# Goose-to-Zed Migration Audit Index

The authoritative migration inventory is [coverage-catalog.md](coverage-catalog.md). It records 152 stable capability IDs, observable behavior, source evidence, existing Zed evidence, requirement/design/task traceability, one exclusive classification, ownership, remaining verification, confidence, and open questions.

[coverage-summary.md](coverage-summary.md) is the decision-oriented roll-up: classification counts, coverage estimates, all missing and partially covered IDs, reuse opportunities, specification overclaims, validation status, and unresolved decisions.

The feature packs beneath this directory are implementation plans only. They do not prove implementation. Every task remains unchecked and must be completed only after its dependencies, product decisions, reads/writes, and validation metadata are satisfied.

The cross-cutting [`agentic` feature boundary](feature-boundary.md) applies to every current and future production write in these packs. The application-owned implementation and validation plan is in [`agentic-feature/`](agentic-feature/requirements.md). A migration leaf cannot be completed until its evidence classifies actual production writes as agentic or feature-neutral and records the inherited enabled/disabled validation.

## Audit rules

1. Observable behavior and failure behavior take precedence over matching names or directories.
2. Executable Zed paths are required for `C1` or `C2`; stubs, declarations, dependencies, docs, and tests without a runtime path do not count.
3. Existing Zed owners are extended before proposing another crate, registry, persistence store, event stream, renderer, scheduler, or service.
4. ACP stdio/HTTP/WebSocket is Goose's current server surface. REST/OpenAPI is not a parity claim.
5. Product/security/operations choices remain explicit decisions rather than implicit implementation tasks.
6. Requirement IDs and acceptance-criterion IDs are stable; design elements and leaf tasks trace back to them.
7. A checked task is prohibited until implementation and its concrete validation have actually completed.
8. `crates/zed` owns the default-enabled `agentic` Cargo feature; no Goose migration registration, dependency, service, background task, tool, permission, command, menu, or network initializer may enter the disabled application graph.
