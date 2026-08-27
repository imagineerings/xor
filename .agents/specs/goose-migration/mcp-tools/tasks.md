# Implementation Plan: MCP Tools

> Cross-cutting contract: every production write in this plan inherits the [`agentic` feature boundary](../feature-boundary.md). Completion evidence must classify actual writes and include the required enabled/disabled validation.

## Overview

Implement the MCP tool servers from goose as a combination of native agent tools and standalone MCP servers. Document tools (PDF, DOCX, XLSX) become native agent tools in `crates/agent/src/tools/`. Memory, Peekaboo, AutoVisualiser, and Tutorial become MCP server binaries that the context server can launch.

## Tasks

- [ ] 1. Reconcile MCP server launch with existing context-server lifecycle
  - Extend existing context-server/agent-server subprocess launch behavior rather than creating a second runner library
  - Handle process lifecycle, stdio transport, health checks, crash recovery

  - _Requirements: 6.1, 6.2, 6.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose-mcp/src/mcp_server_runner.rs, crates/context_server/, crates/agent_servers/_
  - _Writes: selected existing context-server/agent-server lifecycle owner_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement Computer Controller native tools
  - [ ] 2.1. PDF tool — create, read, modify PDFs using `printpdf` or similar
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/tools/platform_
    - _Writes: crates/agent/src/tools/platform_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 2.2. DOCX tool — create, read, modify DOCX files
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/tools/platform_
    - _Writes: crates/agent/src/tools/platform_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 2.3. XLSX tool — create, read, modify XLSX files
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/tools/platform_
    - _Writes: crates/agent/src/tools/platform_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 2.4. Platform automation tool — OS-specific operations

  - _Requirements: 1.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/tools/platform_tool.rs_
  - _Writes: crates/agent/src/tools/platform_tool.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Implement Memory MCP server
  - Match source-confirmed remember, retrieve, remove-category, and remove-specific-memory behavior
  - Preserve explicit project-local versus user-global file scope and deletion behavior; do not invent SQLite parity
  - Add path, permissions, corruption, concurrent update, prompt-disclosure, and privacy controls before enabling persistent memory

  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/mcp_servers/memory/src/lib.rs_
  - _Writes: crates/mcp_servers/memory/src/lib.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement Peekaboo screen capture MCP server
  - [ ] 4.1. Define the approved macOS Peekaboo CLI boundary and explicit unsupported-platform behavior
    - _Requirements: 3.1, 3.2, 3.3_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose-mcp/src/peekaboo/mod.rs, crates/context_server/_
    - _Writes: selected existing context-server/bundled-tool owner_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
    - macOS: invoke and validate the approved Peekaboo CLI contract
    - Windows/Linux: return explicit unsupported-platform behavior unless separately designed and approved
  - [ ] 4.2. MCP server wrapping screen capture as tools

  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/mcp_servers/peekaboo/src/server.rs_
  - _Writes: crates/mcp_servers/peekaboo/src/server.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement AutoVisualiser MCP server
  - Create MCP server with visualization tools
  - Implement template rendering for diagrams (Mermaid, SVG, etc.)

  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/mcp_servers/autovisualiser/src/lib.rs_
  - _Writes: crates/mcp_servers/autovisualiser/src/lib.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Implement Tutorial MCP server
  - Create MCP server that loads and guides through markdown tutorials
  - Implement step progression and state tracking

  - _Requirements: 5.1, 5.2, 5.3, 5.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/mcp_servers/tutorial/src/lib.rs_
  - _Writes: crates/mcp_servers/tutorial/src/lib.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Write tests
  - Document format validation tests (PDF, DOCX, XLSX)
  - Memory persistence and CRUD tests
  - Screen capture tests on each platform
  - Template rendering tests

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/mcp-tools/requirements.md, .agents/specs/goose-migration/mcp-tools/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/mcp_servers/*/tests/_
  - _Writes: crates/mcp_servers/*/tests/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

## Notes

- Native tools register with `crates/agent/` tool registry
- MCP servers are separate binaries launched by `crates/context_server/`
- Each MCP server crate publishes both a library and a binary
