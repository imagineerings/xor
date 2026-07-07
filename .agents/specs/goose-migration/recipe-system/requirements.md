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

1. WHEN a recipe is loaded THEN the system SHALL parse its YAML definition
2. THE recipe SHALL support defining steps with prompts, tools, and conditions
3. THE recipe SHALL support parameterized templates with variable substitution
4. THE recipe SHALL support both local file recipes and built-in recipes
5. WHEN a recipe step completes THEN the system SHALL proceed to the next step
6. IF a recipe step fails THEN the system SHALL handle the error according to the recipe's error policy

### Requirement 2: Recipe Templates

**User Story:** As a sim user, I want to create parameterized recipes with variables, so that I can reuse recipes with different inputs.

#### Acceptance Criteria

1. THE recipe template system SHALL support variable placeholders in recipe files
2. WHEN a templated recipe is run THEN the system SHALL prompt for or accept variable values
3. THE template system SHALL substitute variables before recipe execution

### Requirement 3: Recipe Validation

**User Story:** As a sim user, I want recipes to be validated before execution, so that errors are caught early.

#### Acceptance Criteria

1. WHEN a recipe is loaded THEN the system SHALL validate its structure and required fields
2. IF validation fails THEN the system SHALL report specific validation errors
3. THE validator SHALL check for required fields, correct types, and valid step references

### Requirement 4: Local and Built-in Recipes

**User Story:** As a sim user, I want to load recipes from local files and from built-in sources, so that I can use community recipes and share my own.

#### Acceptance Criteria

1. THE system SHALL discover recipes from a local recipes directory
2. THE system SHALL include built-in recipes shipped with the application
3. WHEN loading a recipe by name THEN the system SHALL search local recipes first, then built-in

### Requirement 5: YAML Recipe Format

**User Story:** As a sim user, I want recipes defined in a clear YAML format, so that they are human-readable and easy to write.

#### Acceptance Criteria

1. THE recipe format SHALL be YAML-based with a defined schema
2. THE recipe format SHALL support steps, prompts, tool configurations, and metadata
3. THE YAML utilities SHALL provide consistent formatting and parsing

### Requirement 6: Recipe CLI Commands

**User Story:** As a sim user, I want CLI commands to manage recipes, so that I can list, search, print, and run recipes from the terminal.

#### Acceptance Criteria

1. THE CLI SHALL provide a command to list available recipes
2. THE CLI SHALL provide a command to search recipes by keyword
3. THE CLI SHALL provide a command to print a recipe's contents
4. THE CLI SHALL provide a command to extract recipes from various sources
5. THE CLI SHALL provide a command to run a recipe

### Requirement 7: GitHub Recipes

**User Story:** As a sim user, I want to load recipes directly from GitHub repositories, so that I can share and discover community recipes.

#### Acceptance Criteria

1. THE system SHALL support loading recipes from GitHub repositories
2. WHEN loading a GitHub recipe THEN the system SHALL fetch the recipe file from the repository
3. IF the GitHub repository or recipe is not found THEN the system SHALL return a clear error

### Requirement 8: Secret Discovery

**User Story:** As a sim user, I want recipes to discover required secrets and credentials, so that I can securely use recipes that need API keys.

#### Acceptance Criteria

1. THE recipe system SHALL detect required secrets defined in recipes
2. WHEN a recipe needs a secret that is not configured THEN the system SHALL prompt the user
3. THE secret discovery SHALL check configured provider credentials and environment variables

### Requirement 9: Recipe Deeplink

**User Story:** As a sim user, I want to share recipe links that automatically open goose with that recipe, so that I can share workflows with others.

#### Acceptance Criteria

1. THE system SHALL support a deeplink URI scheme for recipes
2. WHEN a recipe deeplink is opened THEN the system SHALL load the specified recipe
3. IF the deeplink recipe cannot be found THEN the system SHALL show an appropriate error

### Requirement 10: Recipe Scanner

**User Story:** As a sim developer, I want to scan and test recipes in isolated environments, so that I can ensure recipes work correctly before shipping.

#### Acceptance Criteria

1. THE recipe scanner SHALL support running recipes in Docker containers
2. THE scanner SHALL provide configuration for testing environments
3. THE scanner SHALL output scan results showing success/failure for each recipe

### Requirement 11: Workflow Recipes

**User Story:** As a sim user, I want pre-built workflow recipes for common tasks, so that I can use them out of the box.

#### Acceptance Criteria

1. THE system SHALL ship with pre-built workflow recipes
2. THE workflow recipes SHALL include at least a release risk check recipe
3. THE workflow recipes SHALL be installed alongside the application

## References

- Source: `projects/goose/crates/goose/src/recipe/` — mod.rs, manifest.rs, local_recipes.rs, template_recipe.rs, validate_recipe.rs, yaml_format_utils.rs, read_recipe_file_content.rs, recipe_extension_adapter.rs
- Source: `projects/goose/crates/goose/src/recipe/build_recipe/`
- Source: `projects/goose/crates/goose-cli/src/recipes/` — mod.rs, recipe.rs, extract_from_cli.rs, github_recipe.rs, print_recipe.rs, search_recipe.rs, secret_discovery.rs
- Source: `projects/goose/crates/goose/src/recipe_deeplink.rs`
- Source: `projects/goose/recipe-scanner/`
- Source: `projects/goose/workflow_recipes/`
