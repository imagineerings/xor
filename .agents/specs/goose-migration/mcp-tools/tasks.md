# Implementation Plan: MCP Tools

## Overview

Implement the MCP tool servers from goose as a combination of native agent tools and standalone MCP servers. Document tools (PDF, DOCX, XLSX) become native agent tools in `crates/agent/src/tools/`. Memory, Peekaboo, AutoVisualiser, and Tutorial become MCP server binaries that the context server can launch.

## Tasks

- [x] 1. Create shared MCP server runner library
  - Extract the subprocess MCP server runner pattern into a shared utility
  - Handle process lifecycle, stdio transport, health checks, crash recovery
  - _Requirements: 6_
  - _writes: crates/mcp_runner/src/lib.rs_

- [ ] 2. Implement Computer Controller native tools
  - [ ] 2.1 PDF tool — create, read, modify PDFs using `printpdf` or similar
    - _Requirements: 1.1_
    - _writes: crates/agent/src/tools/pdf_tool.rs_
  - [ ] 2.2 DOCX tool — create, read, modify DOCX files
    - _Requirements: 1.2_
    - _writes: crates/agent/src/tools/docx_tool.rs_
  - [ ] 2.3 XLSX tool — create, read, modify XLSX files
    - _Requirements: 1.3_
    - _writes: crates/agent/src/tools/xlsx_tool.rs_
  - [ ] 2.4 Platform automation tool — OS-specific operations
    - _Requirements: 1.4_
    - _writes: crates/agent/src/tools/platform_tool.rs_

- [x] 3. Implement Memory MCP server
  - Create MCP server with `store_fact`, `retrieve_memories`, `search_memories` tools
  - Implement local SQLite-based memory store
  - Add persistence across restarts
  - _Requirements: 2_
  - _writes: crates/mcp_servers/memory/src/lib.rs_

- [ ] 4. Implement Peekaboo screen capture MCP server
  - [ ] 4.1 Screen capture abstraction with platform backends
    - macOS: ScreenCaptureKit
    - Windows: DXGI
    - Linux: X11/PipeWire
    - _Requirements: 3.1_
    - _writes: crates/mcp_servers/peekaboo/src/lib.rs_
  - [ ] 4.2 MCP server wrapping screen capture as tools
    - _Requirements: 3_
    - _writes: crates/mcp_servers/peekaboo/src/server.rs_

- [x] 5. Implement AutoVisualiser MCP server
  - Create MCP server with visualization tools
  - Implement template rendering for diagrams (Mermaid, SVG, etc.)
  - _Requirements: 4_
  - _writes: crates/mcp_servers/autovisualiser/src/lib.rs_

- [x] 6. Implement Tutorial MCP server
  - Create MCP server that loads and guides through markdown tutorials
  - Implement step progression and state tracking
  - _Requirements: 5_
  - _writes: crates/mcp_servers/tutorial/src/lib.rs_

- [ ] 7. Write tests
  - Document format validation tests (PDF, DOCX, XLSX)
  - Memory persistence and CRUD tests
  - Screen capture tests on each platform
  - Template rendering tests
  - _Requirements: 1-6_
  - _writes: crates/mcp_servers/*/tests/_

## Notes

- Native tools register with `crates/agent/` tool registry
- MCP servers are separate binaries launched by `crates/context_server/`
- Each MCP server crate publishes both a library and a binary
