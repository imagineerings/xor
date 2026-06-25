# Requirements: Security and Permissions

## Introduction

Migrate the comprehensive security and permission systems from goose. The security system provides adversarial input inspection, egress data inspection, content classification, pattern-based threat detection, and a unified security scanner. The permission system provides user-in-the-loop permission confirmation, inspection, judgment, and persistent permission storage.

## Glossary

- **Adversary Inspector**: Checks incoming data (prompts, tool inputs) for prompt injection and adversarial content
- **Egress Inspector**: Checks outgoing data (tool outputs, responses) for sensitive data leakage
- **Classification Client**: Client for content classification/safety APIs
- **Security Scanner**: Unified scanner that coordinates all security checks
- **Security Inspector**: Top-level orchestrator for security inspection
- **Pattern**: Known patterns for detecting threats or sensitive data
- **Permission Confirmation**: UI dialog that asks user to confirm an action
- **Permission Inspector**: Examines tool calls to determine if they need permission
- **Permission Judge**: Evaluates whether a tool call should be allowed
- **Permission Store**: Persistent storage for permission decisions (allow/deny/always-allow)

## Requirements

### Requirement 1: Adversary Inspector

**User Story:** As a baymax user, I want protection against prompt injection and adversarial inputs, so that malicious instructions cannot hijack the agent.

#### Acceptance Criteria

1. WHEN input is received (prompt, tool arguments) THEN the adversary inspector SHALL check for prompt injection patterns
2. IF adversarial content is detected THEN the system SHALL block the input and notify the user
3. THE adversary inspector SHALL support configurable sensitivity levels

### Requirement 2: Egress Inspector

**User Story:** As a baymax user, I want protection against sensitive data leakage in agent outputs, so that confidential information is not accidentally sent to external services.

#### Acceptance Criteria

1. WHEN the agent produces output that will be sent externally THEN the egress inspector SHALL check for sensitive data
2. IF sensitive data patterns are detected THEN the system SHALL block or redact the output
3. THE egress inspector SHALL support configurable patterns for what constitutes sensitive data

### Requirement 3: Classification Client

**User Story:** As a baymax user, I want content classification for safety, so that harmful content can be identified and blocked.

#### Acceptance Criteria

1. THE system SHALL provide a classification client that calls content moderation APIs
2. WHEN content is submitted for classification THEN the client SHALL return safety ratings
3. IF the content exceeds safety thresholds THEN the system SHALL take configured action

### Requirement 4: Security Scanner

**User Story:** As a baymax user, I want a unified security scanner that coordinates all security checks, so that security is consistently applied.

#### Acceptance Criteria

1. THE security scanner SHALL coordinate adversary inspection, egress inspection, and classification
2. WHEN content is scanned THEN all configured inspectors SHALL run
3. IF any inspector flags the content THEN the scanner SHALL report the aggregated results

### Requirement 5: Security Patterns

**User Story:** As a baymax developer, I want a pattern system for threat detection, so that I can define and update security patterns without code changes.

#### Acceptance Criteria

1. THE pattern system SHALL support regex and other pattern types
2. THE pattern system SHALL support pattern categories (injection, sensitive data, etc.)
3. WHEN content is checked against patterns THEN all active patterns SHALL be evaluated

### Requirement 6: Permission Confirmation

**User Story:** As a baymax user, I want to confirm sensitive actions before the agent performs them, so that I maintain control over critical operations.

#### Acceptance Criteria

1. WHEN the agent wants to perform a sensitive action THEN the system SHALL show a permission confirmation dialog
2. THE confirmation dialog SHALL show the action details, tool name, and arguments
3. THE user SHALL be able to allow, deny, or always-allow the action

### Requirement 7: Permission Inspector

**User Story:** As a baymax user, I want the system to examine tool calls against permission policies, so that actions are automatically classified by risk level.

#### Acceptance Criteria

1. THE permission inspector SHALL examine each tool call before execution
2. THE inspector SHALL classify the action's risk level based on tool, arguments, and context
3. THE inspector SHALL check against stored permission decisions

### Requirement 8: Permission Judge

**User Story:** As a baymax user, I want an intelligent permission judge that can make automatic decisions for low-risk actions, so that I am not bothered with unnecessary confirmations.

#### Acceptance Criteria

1. THE permission judge SHALL evaluate tool calls against configured policies
2. THE judge SHALL automatically allow low-risk actions based on policy
3. THE judge SHALL escalate medium-risk actions to the user for confirmation
4. THE judge SHALL automatically block high-risk actions

### Requirement 9: Permission Store

**User Story:** As a baymax user, I want permission decisions to be remembered, so that I am not repeatedly asked about the same action.

#### Acceptance Criteria

1. THE permission store SHALL persistently record user permission decisions
2. WHEN a tool call matches a stored "always allow" decision THEN the system SHALL skip confirmation
3. WHEN a tool call matches a stored "always deny" decision THEN the system SHALL block it silently
4. THE permission store SHALL support clearing stored decisions

## References

- Source: `projects/goose/crates/goose/src/security/` — adversary_inspector.rs, egress_inspector.rs, classification_client.rs, scanner.rs, patterns.rs, security_inspector.rs
- Source: `projects/goose/crates/goose/src/permission/` — permission_confirmation.rs, permission_inspector.rs, permission_judge.rs, permission_store.rs
- Existing baymax: `crates/agent/src/tool_permissions.rs`, `crates/sandbox/`
