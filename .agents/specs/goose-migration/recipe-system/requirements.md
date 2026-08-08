# Requirements: Recipe System

## Introduction

Migrate the goose recipe engine — a system for defining, templating, validating, scanning, and executing reusable "recipes" that automate multi-step workflows. This includes the core engine, CLI commands, the recipe scanner, workflow recipes, and recipe deep-linking.

## Glossary

- **Recipe**: A reusable, YAML-defined workflow that guides the agent through structured multi-step tasks
- **Recipe Template**: A parameterized recipe with variables that get substituted at runtime
- **Recipe Scanner**: Infrastructure for scanning and testing recipes in isolated environments (Docker)
- **Workflow Recipe**: A pre-built recipe for specific workflows (e.g., release risk check)
- **Recipe Deeplink**: A URI scheme that opens goose with a specific recipe pre-loaded
- **Recipe Extension Adapter**: Bridge between the recipe system and MCP extensions

## Requirements

### Requirement 1: Recipe Engine

**User Story:** As a sim user, I want to create and run recipes, so that I can automate complex multi-step workflows with the agent.

#### Acceptance Criteria

1. **1.1** WHEN a recipe is loaded THEN the system SHALL parse its YAML definition
2. **1.2** THE recipe SHALL support defining steps with prompts, tools, and conditions
3. **1.3** THE recipe SHALL support parameterized templates with variable substitution
4. **1.4** THE recipe SHALL support both local file recipes and built-in recipes
5. **1.5** WHEN a recipe step completes THEN the system SHALL proceed to the next step
6. **1.6** IF a recipe step fails THEN the system SHALL handle the error according to the recipe's error policy

### Requirement 2: Recipe Templates

**User Story:** As a sim user, I want to create parameterized recipes with variables, so that I can reuse recipes with different inputs.

#### Acceptance Criteria

1. **2.1** THE recipe template system SHALL support variable placeholders in recipe files
2. **2.2** WHEN a templated recipe is run THEN the system SHALL prompt for or accept variable values
3. **2.3** THE template system SHALL substitute variables before recipe execution

### Requirement 3: Recipe Validation

**User Story:** As a sim user, I want recipes to be validated before execution, so that errors are caught early.

#### Acceptance Criteria

1. **3.1** WHEN a recipe is loaded THEN the system SHALL validate its structure and required fields
2. **3.2** IF validation fails THEN the system SHALL report specific validation errors
3. **3.3** THE validator SHALL check for required fields, correct types, and valid step references

### Requirement 4: Local and Built-in Recipes

**User Story:** As a sim user, I want to load recipes from local files and from built-in sources, so that I can use community recipes and share my own.

#### Acceptance Criteria

1. **4.1** THE system SHALL discover recipes from a local recipes directory
2. **4.2** THE system SHALL include built-in recipes shipped with the application
3. **4.3** WHEN loading a recipe by name THEN the system SHALL search local recipes first, then built-in

### Requirement 5: YAML Recipe Format

**User Story:** As a sim user, I want recipes defined in a clear YAML format, so that they are human-readable and easy to write.

#### Acceptance Criteria

1. **5.1** THE recipe format SHALL be YAML-based with a defined schema
2. **5.2** THE recipe format SHALL support steps, prompts, tool configurations, and metadata
3. **5.3** THE YAML utilities SHALL provide consistent formatting and parsing

### Requirement 6: Recipe CLI Commands

**User Story:** As a sim user, I want CLI commands to manage recipes, so that I can list, search, print, and run recipes from the terminal.

#### Acceptance Criteria

1. **6.1** THE CLI SHALL provide a command to list available recipes
2. **6.2** THE CLI SHALL provide a command to search recipes by keyword
3. **6.3** THE CLI SHALL provide a command to print a recipe's contents
4. **6.4** THE CLI SHALL provide a command to extract recipes from various sources
5. **6.5** THE CLI SHALL provide a command to run a recipe

### Requirement 7: GitHub Recipes

**User Story:** As a sim user, I want to load recipes directly from GitHub repositories, so that I can share and discover community recipes.

#### Acceptance Criteria

1. **7.1** THE system SHALL support loading recipes from GitHub repositories
2. **7.2** WHEN loading a GitHub recipe THEN the system SHALL fetch the recipe file from the repository
3. **7.3** IF the GitHub repository or recipe is not found THEN the system SHALL return a clear error

### Requirement 8: Secret Discovery

**User Story:** As a sim user, I want recipes to discover required secrets and credentials, so that I can securely use recipes that need API keys.

#### Acceptance Criteria

1. **8.1** THE recipe system SHALL detect required secrets defined in recipes
2. **8.2** WHEN a recipe needs a secret that is not configured THEN the system SHALL prompt the user
3. **8.3** THE secret discovery SHALL check configured provider credentials and environment variables

### Requirement 9: Recipe Deeplink

**User Story:** As a sim user, I want to share recipe links that automatically open goose with that recipe, so that I can share workflows with others.

#### Acceptance Criteria

1. **9.1** THE system SHALL support a deeplink URI scheme for recipes
2. **9.2** WHEN a recipe deeplink is opened THEN the system SHALL load the specified recipe
3. **9.3** IF the deeplink recipe cannot be found THEN the system SHALL show an appropriate error

### Requirement 10: Recipe Scanner

**User Story:** As a sim developer, I want to scan and test recipes in isolated environments, so that I can ensure recipes work correctly before shipping.

#### Acceptance Criteria

1. **10.1** THE recipe scanner SHALL support running recipes in Docker containers
2. **10.2** THE scanner SHALL provide configuration for testing environments
3. **10.3** THE scanner SHALL output scan results showing success/failure for each recipe

### Requirement 11: Workflow Recipes

**User Story:** As a sim user, I want pre-built workflow recipes for common tasks, so that I can use them out of the box.

#### Acceptance Criteria

1. **11.1** THE system SHALL ship with pre-built workflow recipes
2. **11.2** THE workflow recipes SHALL include at least a release risk check recipe
3. **11.3** THE workflow recipes SHALL be installed alongside the application

### Requirement 12: Sub-Recipe Composition

**User Story:** As a recipe author, I want recipes to compose other recipes safely, so that reusable workflows do not duplicate definitions or recurse indefinitely.

#### Acceptance Criteria

1. **12.1** THE recipe model SHALL support named sub-recipes with relative or absolute source paths and explicit parameter values
2. **12.2** THE builder SHALL resolve nested paths relative to the declaring recipe, preserve deterministic declaration order, and define parameter override precedence
3. **12.3** IF a sub-recipe is missing, duplicated incompatibly, cyclic, or exceeds the configured nesting limit, THEN validation SHALL fail before agent execution with the dependency chain
4. **12.4** CLI-supplied additional sub-recipes SHALL pass through the same validation, secret discovery, and composition rules

### Requirement 13: Complete Recipe Command Surface

**User Story:** As a terminal user, I want source-compatible recipe inspection and launch commands, so that I can validate and understand a workflow before running it.

#### Acceptance Criteria

1. **13.1** WHERE recipe CLI parity is approved, THE CLI SHALL support validate, list/search, print/explain, render, open/deeplink, run, and parameter inspection through the shared recipe service
2. **13.2** THE run command SHALL accept documented stdin/file instructions, typed parameters, additional sub-recipes, interactive continuation, and machine-readable output behavior
3. **13.3** EACH command SHALL have deterministic stdout/stderr separation, exit codes, noninteractive missing-input behavior, and trust prompts for remote or deeplink sources

### Requirement 14: Scheduled Recipe Service and Agent Tool

**User Story:** As a user, I want approved recipes to run on a persisted schedule, so that recurring work survives restarts and remains controllable.

#### Acceptance Criteria

1. **14.1** WHERE scheduling is approved, ONE recipe/session service SHALL persist cron jobs, timezone, pause state, recipe snapshot/reference, generated session IDs, and last/next run state across restarts
2. **14.2** THE service SHALL support list, add, remove, pause, unpause, run now, list generated sessions, inspect running work, and cancel a running job through shared CLI, ACP, and UI adapters
3. **14.3** THE scheduler SHALL define DST, missed-run, overlap, retry, restart-recovery, recipe-change, notification, and cleanup behavior
4. **14.4** AN optional agent schedule tool SHALL use the same service, validate cron and bounded recipe input, apply Sim permission confirmation, exclude or redact secrets, and emit an audit record

## References

- Source: `projects/goose/crates/goose/src/recipe/` — mod.rs, manifest.rs, local_recipes.rs, template_recipe.rs, validate_recipe.rs, yaml_format_utils.rs, read_recipe_file_content.rs, recipe_extension_adapter.rs
- Source: `projects/goose/crates/goose/src/recipe/build_recipe/`
- Source: `projects/goose/crates/goose-cli/src/recipes/` — mod.rs, recipe.rs, extract_from_cli.rs, github_recipe.rs, print_recipe.rs, search_recipe.rs, secret_discovery.rs
- Source: `projects/goose/crates/goose/src/recipe_deeplink.rs`
- Source: `projects/goose/recipe-scanner/`
- Source: `projects/goose/workflow_recipes/`
- Source: `projects/goose/crates/goose/src/agents/schedule_tool.rs`, `scheduler.rs`, `scheduler_trait.rs`
