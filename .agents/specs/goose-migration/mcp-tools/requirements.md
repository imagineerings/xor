# Requirements: MCP Tools

## Introduction

Migrate the Model Context Protocol (MCP) tools from goose that provide desktop automation, visualization, memory, screen monitoring, and tutorials. These are standalone MCP server implementations that goose uses as extensions.

## Glossary

- **MCP**: Model Context Protocol — open standard for AI agent tool/extension communication
- **MCP Server**: A process that implements the MCP protocol to expose tools, resources, and prompts
- **Computer Controller**: MCP server for desktop automation (file operations, document creation)
- **AutoVisualiser**: MCP server for code visualization and diagramming
- **Memory**: MCP server for long-term memory storage and retrieval
- **Peekaboo**: MCP server for screen/content monitoring

## Requirements

### Requirement 1: Computer Controller MCP Tools

**User Story:** As a sim user, I want the agent to create and manipulate documents (PDF, DOCX, XLSX) and control computer platforms, so that I can automate document workflows.

#### Acceptance Criteria

1. **1.1** THE system SHALL provide an MCP server with tools for PDF operations (create, read, modify)
2. **1.2** THE system SHALL provide an MCP server with tools for DOCX operations (create, read, modify)
3. **1.3** THE system SHALL provide an MCP server with tools for XLSX operations (create, read, modify)
4. **1.4** THE computer controller SHALL dispatch to the source-confirmed macOS, Linux, and Windows platform modules and SHALL report unsupported operations per platform rather than claiming uniform support
5. **1.5** WHEN a document tool is invoked THEN the system SHALL produce the expected file format
6. **1.6** IF the document file cannot be created THEN the system SHALL return a descriptive error

### Requirement 2: Memory MCP Tool

**User Story:** As a sim user, I want the agent to have persistent long-term memory, so that it can recall information across sessions.

#### Acceptance Criteria

1. **2.1** THE system SHALL provide an MCP server for long-term memory storage
2. **2.2** THE memory system SHALL support storing structured facts and information
3. **2.3** THE memory system SHALL support retrieving relevant memories based on query
4. **2.4** WHEN information is stored in memory THEN it SHALL persist across agent sessions
5. **2.5** THE memory data SHALL be stored locally with explicit project-local versus user-global scope, user-visible deletion, path traversal protection, and no implicit transmission to providers except through disclosed prompt context

### Requirement 3: Peekaboo Screen Monitoring

**User Story:** As a sim user, I want the agent to be able to capture and analyze screen content, so that it can help with visual tasks.

#### Acceptance Criteria

1. **3.1** ON macOS, WHERE Peekaboo support is approved and the Peekaboo CLI is available, THE system SHALL provide the source-compatible screen and GUI-automation tools
2. **3.2** THE integration SHALL validate binary availability, screen-recording/accessibility permissions, display selection, and requested capture target before execution
3. **3.3** IF the platform is not macOS or capture/automation fails, THEN the system SHALL return a clear unsupported-platform, missing-binary, permission, display, timeout, or subprocess error

### Requirement 4: AutoVisualiser

**User Story:** As a sim user, I want the agent to generate visual diagrams and visualizations from code or descriptions, so that I can understand complex systems.

#### Acceptance Criteria

1. **4.1** WHERE embedded MCP Apps are approved, THE AutoVisualiser SHALL expose chart, chord, donut, map, Mermaid, radar, Sankey, and treemap visualizations through MCP resources
2. **4.2** THE visualizer SHALL validate each visualization payload against its declared schema and return descriptive errors for unsupported types or malformed data
3. **4.3** THE visualizer SHALL use bundled, versioned templates and assets without fetching executable code from arbitrary network origins
4. **4.4** WHEN visualization is requested THEN the system SHALL produce an isolated interactive MCP App resource or an explicitly supported static export, and SHALL apply the shared MCP Apps security policy

### Requirement 5: Tutorial MCP Tool

**User Story:** As a sim user, I want the agent to provide interactive tutorials, so that I can learn how to use features step by step.

#### Acceptance Criteria

1. **5.1** THE system SHALL provide an MCP server for tutorials
2. **5.2** THE tutorial system SHALL load tutorials from markdown files
3. **5.3** THE tutorial system SHALL guide users through step-by-step instructions
4. **5.4** WHEN a tutorial step is completed THEN the system SHALL advance to the next step

### Requirement 6: MCP Server Runner

**User Story:** As a sim developer, I want a standardized MCP server runner, so that MCP tools can be launched and managed consistently.

#### Acceptance Criteria

1. **6.1** THE system SHALL provide an MCP server runner for launching subprocess-based MCP servers
2. **6.2** THE MCP server runner SHALL handle process lifecycle (start, health-check, stop)
3. **6.3** IF an MCP server process crashes THEN the system SHALL report the failure

## References

- Source: `projects/goose/crates/goose-mcp/src/` — autovisualiser/, computercontroller/, memory/, peekaboo/, tutorial/, mcp_server_runner.rs, subprocess.rs
- Existing sim: `crates/context_server/` (MCP client), `crates/agent_servers/` (agent server management)
