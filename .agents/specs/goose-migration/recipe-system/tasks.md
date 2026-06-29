# Implementation Plan: Recipe System

## Overview

Implement the recipe engine as a new `crates/recipe/` crate with YAML-based recipe definitions, template engine, validator, discovery sources (local, builtin, GitHub), and CLI commands. Also implement the recipe scanner (Docker-based) and ship workflow recipes.

## Tasks

- [x] 1. Create `crates/recipe/` crate with core data structures
  - Define Recipe, RecipeStep, VariableDefinition, RecipeManifest types
  - Implement YAML serialization/deserialization with schema validation
  - _Requirements: 1.1, 5_
  - _writes: crates/recipe/src/lib.rs, crates/recipe/src/types.rs_

- [x] 2. Implement recipe YAML format utilities
  - Consistent YAML formatting, parsing with error context
  - Schema validation for required fields and types
  - _Requirements: 5_
  - _writes: crates/recipe/src/yaml_format.rs, crates/recipe/src/validator.rs_

- [x] 3. Implement template engine
  - Variable substitution with `{{ variable }}` syntax
  - Template validation (detect missing variables)
  - Template composition (nested templates)
  - _Requirements: 2_
  - _writes: crates/recipe/src/template.rs_

- [x] 4. Implement recipe sources
  - [x] 4.1 Local recipe source — discover and load from filesystem directory
    - _Requirements: 4.1_
    - _writes: crates/recipe/src/sources/local.rs_
  - [x] 4.2 Builtin recipe source — embed recipes in the binary
    - _Requirements: 4.2_
    - _writes: crates/recipe/src/sources/builtin.rs_
  - [x] 4.3 GitHub recipe source — fetch recipes from GitHub repositories
    - _Requirements: 7_
    - _writes: crates/recipe/src/sources/github.rs_

- [x] 5. Implement recipe engine
  - Recipe discovery across all registered sources
  - Recipe loading with deduplication (local overrides builtin)
  - Recipe execution with step sequencing, error policies, and variable injection
  - _Requirements: 1_
  - _writes: crates/recipe/src/engine.rs, crates/recipe/src/execution.rs_

- [x] 6. Implement recipe deeplink handler
  - Parse recipe deeplink URIs
  - Resolve and load recipe from deeplink
  - _Requirements: 9_
  - _writes: crates/recipe/src/deeplink.rs_

- [x] 7. Implement recipe CLI commands
  - [x] 7.1 `goose recipe list` — list available recipes
    - _Requirements: 6.1_
    - _writes: crates/cli/src/commands/recipe.rs_
  - [x] 7.2 `goose recipe search` — search recipes by keyword
    - _Requirements: 6.2_
    - _writes: crates/cli/src/commands/recipe.rs_
  - [x] 7.3 `goose recipe print` — print recipe contents
    - _Requirements: 6.3_
    - _writes: crates/cli/src/commands/recipe.rs_
  - [x] 7.4 `goose recipe run` — execute a recipe
    - _Requirements: 6.4_
    - _writes: crates/cli/src/commands/recipe.rs_

- [x] 8. Implement GitHub recipe and secret discovery
  - GitHub recipe fetching with caching
  - Secret/variable discovery — detect required secrets, check configuration
  - _Requirements: 7, 8_
  - _writes: crates/recipe/src/sources/github.rs, crates/recipe/src/secrets.rs_

- [x] 9. Implement recipe scanner (Docker-based)
  - Docker image with recipe testing environment
  - Scan script that runs each recipe and checks output
  - Result reporting
  - _Requirements: 10_
  - _writes: recipe-scanner/Dockerfile, recipe-scanner/scan.sh_

- [x] 10. Ship workflow recipes
  - Create release risk check recipe
  - Add recipe installation path in the application
  - _Requirements: 11_
  - _writes: crates/recipe/src/builtin_recipes/_

- [x] 11. Write tests
  - Unit tests: YAML parsing, template rendering, validation
  - Integration tests: Full recipe execution with mock agent
  - CLI tests: All recipe subcommands
  - _Requirements: 1-11_
  - _writes: crates/recipe/tests/_

## Notes

- Recipe engine is a library; the CLI and desktop UI are consumers
- Built-in recipes ship inside the binary via `include_dir!`
- Scanner is a separate deployment artifact (Docker-based)
