# Goose migration agentic feature boundary

This document is the cross-cutting compile-time contract for every Goose migration task. `crates/zed` owns the default-enabled `agentic` Cargo feature. Every production write made by this portfolio must be reviewed under one of the classifications below.

## Task authoring rule

For every existing or future leaf that writes production code or configuration:

1. Mark the affected module, dependency, registration, command, menu, service, background task, permission, tool, or network initializer as **agentic**, and name the `agentic` feature/module boundary plus disabled-build absence validation; or
2. Mark the write as **feature-neutral**, explain its independently useful non-agent behavior, and name the gated agent consumer/adapter plus disabled-build validation.

Documentation, fixtures, tests, and developer tooling must state which product they validate. A new external service or SDK is outside the desktop dependency graph, but every desktop launcher, discovery hook, credential flow, command, menu, or network registration for it is agentic and must be excluded from the disabled application.

Unchecked tasks inherit this contract even when their historical `_Writes:` metadata names a directory or a path that has not been selected yet. Before implementation, the leaf must narrow its real write paths and record the classification in its evidence.

## Existing production-write classification

| Migration pack | Classification of existing write families | Required compile-time boundary |
| --- | --- | --- |
| `agent-infrastructure` | Agentic | Pure agent/ACP crates are optional in `zed`; shared diagnostics, HTTP, telemetry, container, and prompt primitives are feature-neutral only when their agent adapter and registration are gated. |
| `additional-llm-providers` | Agentic registrations with potentially feature-neutral provider primitives | Provider implementations used only by agent conversations are optional/gated; common model metadata may remain feature-neutral when no agent registry or network initializer is registered without `agentic`. |
| `auth` | Feature-neutral credential/OAuth primitives with agentic provider/MCP flows | Shared credential parsing/storage may compile; agent provider callbacks, discovery, commands, services, and network initialization require `agentic`. |
| `desktop-ui` | Agentic | Agent panels, settings pages, schedules, imports, context renderers, menus, actions, and update announcements are compiled and registered only with `agentic`. |
| `developer-experience` | Agentic | Agent commands, skills, prompt/AGENTS.md watchers, thread lifecycle, and agent UI editors require `agentic`; generic filesystem parsing is neutral only behind a gated consumer. |
| `dictation` | Feature-neutral audio capture plus agentic dictation product | Generic audio capture may compile; dictation services/providers/tools/commands and their network initialization require `agentic`. |
| `documentation` | Feature-neutral artifacts describing two products | Examples and generated references must state feature requirements; agent commands are never presented as available in a disabled build. |
| `evaluation` | Agentic tooling not linked into disabled `zed` | Agent scenarios, mock providers, benchmarks, and runners remain separate targets; no disabled desktop registration or dependency edge is permitted. |
| `gateway` | Agentic | Gateway services, Telegram adapters, pairing, credentials, commands, and network tasks require `agentic` and are absent from disabled `zed`. |
| `goal-grind-commands` | Agentic | Goal persistence, grind execution, actions, commands, UI, and background work require `agentic`; generic task primitives may remain neutral behind gated adapters. |
| `mcp-tools` | Agentic | MCP servers/tools, context registration, app renderers, permissions, and processes require `agentic`; generic protocol data types may be neutral only without registration. |
| `misc-services` | Mixed | Agent session import/share adapters are agentic. Standalone examples, scripts, proxies, and approved external services are not linked into `zed`; any desktop discovery, launch, credential, or network hook requires `agentic`. |
| `observability-analytics` | Feature-neutral telemetry primitives with agentic producers/exporters | Generic token/rate/telemetry infrastructure may compile; agent events, inspectors, exporters configured solely for agents, settings, and network initialization require `agentic`. |
| `recipe-system` | Agentic | Recipe models, sources, engine, schedules, deeplinks, secrets, commands, UI, and network fetches require `agentic`; generic scheduler primitives may remain neutral behind a gated adapter. |
| `rest-api-server` | Agentic | ACP server transports, custom methods, CLI launch, credentials, listeners, and network initialization require `agentic`. |
| `security-permissions` | Feature-neutral security primitives with agentic policy pipeline | Shared scanners/classifiers may compile only when independently used; agent inspectors, permission persistence/UI, egress calls, and tool decisions require `agentic`. |
| `text-ui` | Agentic separate target | Interactive agent CLI/TUI commands, configuration, providers, onboarding, renderers, and network sessions require an `agentic` target feature and are never pulled into disabled `zed`. |
| `typescript-sdk` | Agentic external artifact | SDK packages are separately built artifacts; desktop SDK discovery, binary resolution, process launch, or network service registration requires `agentic`. |

## Validation matrix inherited by all packs

- Agentic production leaves: focused enabled tests plus disabled compile/graph/registration absence checks.
- Feature-neutral leaves: focused tests without `agentic`, plus an audit proving the agent consumer or adapter is gated.
- Dependency additions: resolved-tree checks proving agent-only packages are absent from `cargo tree -p zed --no-default-features -e features`.
- Workspace feature additions: feature-unification checks proving no participating crate's `agentic` feature is enabled in the disabled graph.
- Persisted formats/actions/URLs: disabled compatibility tests proving explicit safe rejection or preservation without semantic fallback.
