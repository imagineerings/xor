# Implementation Plan: Recipe System

> Cross-cutting contract: every production write in this plan inherits the [`agentic` feature boundary](../feature-boundary.md). Completion evidence must classify actual writes and include the required enabled/disabled validation.

## Overview

Add recipe behavior at the narrowest existing agent/session integration point, separating it into a crate only if implementation review shows multiple existing consumers need a stable library boundary. Reuse Zed's prompt, credentials, settings, git/HTTP, deeplink, session, permission, and executor services.

## Tasks

- [ ] 1. Reconcile ownership and add the core recipe model
  - Inspect existing agent/session/settings integration points and record why the selected owner is narrower than a new subsystem
  - Define Recipe, RecipeStep, VariableDefinition, RecipeManifest types
  - Implement YAML serialization/deserialization with schema validation

  - _Requirements: 1.1, 5.1, 5.2, 5.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/lib.rs, crates/recipe/src/types.rs_
  - _Writes: crates/recipe/src/lib.rs, crates/recipe/src/types.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement recipe YAML format utilities
  - Consistent YAML formatting, parsing with error context
  - Schema validation for required fields and types

  - _Requirements: 5.1, 5.2, 5.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/yaml_format.rs, crates/recipe/src/validator.rs_
  - _Writes: crates/recipe/src/yaml_format.rs, crates/recipe/src/validator.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Implement template engine
  - Variable substitution with `{{ variable }}` syntax
  - Template validation (detect missing variables)
  - Template composition (nested templates)

  - _Requirements: 2.1, 2.2, 2.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/template.rs_
  - _Writes: crates/recipe/src/template.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement recipe sources
  - [ ] 4.1. Local recipe source — discover and load from filesystem directory
    - _Requirements: 7.1, 7.2, 7.3_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/sources_
    - _Writes: crates/recipe/src/sources_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 4.2. Builtin recipe source — embed recipes in the binary
    - _Requirements: 7.1, 7.2, 7.3_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/sources_
    - _Writes: crates/recipe/src/sources_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 4.3. GitHub recipe source — fetch recipes from GitHub repositories

  - _Requirements: 7.1, 7.2, 7.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/sources/github.rs_
  - _Writes: crates/recipe/src/sources/github.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement recipe engine
  - Recipe discovery across all registered sources
  - Recipe loading with deduplication (local overrides builtin)
  - Recipe execution with step sequencing, error policies, and variable injection

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/engine.rs, crates/recipe/src/execution.rs_
  - _Writes: crates/recipe/src/engine.rs, crates/recipe/src/execution.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Implement recipe deeplink handler
  - Parse recipe deeplink URIs
  - Resolve and load recipe from deeplink

  - _Requirements: 9.1, 9.2, 9.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/deeplink.rs_
  - _Writes: crates/recipe/src/deeplink.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Implement recipe CLI commands
  - [ ] 7.1. `goose recipe list` — list available recipes
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/commands_
    - _Writes: crates/cli/src/commands_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 7.2. `goose recipe search` — search recipes by keyword
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/commands_
    - _Writes: crates/cli/src/commands_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 7.3. `goose recipe print` — print recipe contents
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/commands_
    - _Writes: crates/cli/src/commands_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 7.4. `goose recipe run` — execute a recipe

  - _Requirements: 6.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/commands/recipe.rs_
  - _Writes: crates/cli/src/commands/recipe.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 8. Implement GitHub recipe and secret discovery
  - GitHub recipe fetching with caching
  - Secret/variable discovery — detect required secrets, check configuration

  - _Requirements: 7.1, 7.2, 7.3, 8.1, 8.2, 8.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/sources/github.rs, crates/recipe/src/secrets.rs_
  - _Writes: crates/recipe/src/sources/github.rs, crates/recipe/src/secrets.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 9. Implement recipe scanner (Docker-based)
  - Docker image with recipe testing environment
  - Scan script that runs each recipe and checks output
  - Result reporting

  - _Requirements: 10.1, 10.2, 10.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, recipe-scanner/Dockerfile, recipe-scanner/scan.sh_
  - _Writes: recipe-scanner/Dockerfile, recipe-scanner/scan.sh_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 10. Ship workflow recipes
  - Create release risk check recipe
  - Add recipe installation path in the application

  - _Requirements: 11.1, 11.2, 11.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/src/builtin_recipes/_
  - _Writes: crates/recipe/src/builtin_recipes/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 11. Write tests
  - Unit tests: YAML parsing, template rendering, validation
  - Integration tests: Full recipe execution with mock agent
  - CLI tests: All recipe subcommands

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 4.1, 4.2, 4.3, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 9.1, 9.2, 9.3, 10.1, 10.2, 10.3, 11.1, 11.2, 11.3, 12.1, 12.2, 12.3, 12.4, 13.1, 13.2, 13.3, 14.1, 14.2, 14.3, 14.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/recipe/tests/_
  - _Writes: crates/recipe/tests/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 12. Implement sub-recipe graph validation and composition
  - Resolve child paths relative to each declaring recipe and preserve deterministic order and override precedence
  - Detect missing, duplicate-incompatible, cyclic, and over-depth graphs before agent execution
  - Apply the same graph to secret discovery and CLI-supplied additional sub-recipes

  - _Requirements: 12.1, 12.2, 12.3, 12.4_
  - _Depends on: 1, 2, 3, 8_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, projects/goose/crates/goose/src/recipe/build_recipe/, projects/goose/crates/goose-cli/src/recipes/extract_from_cli.rs, projects/goose/crates/goose-cli/src/recipes/secret_discovery.rs_
  - _Writes: selected recipe model and builder owner_
  - _Validation: Run relative/absolute path, order, override, missing child, duplicate, cycle-chain, depth-limit, additional-child, and recursive-secret tests_

- [ ] 13. Complete the approved recipe CLI contract
  - Add validate, print/explain, render, open/deeplink, parameter inspection, and complete run input behavior omitted by the earlier CLI tasks
  - Route every command through the shared recipe service and the text-UI machine-output contract

  - _Requirements: 13.1, 13.2, 13.3_
  - _Depends on: 5, 6, 7, 12, text-ui/9, text-ui/12_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, projects/goose/crates/goose-cli/src/cli.rs, projects/goose/crates/goose-cli/src/commands/recipe.rs, projects/goose/crates/goose-cli/src/recipes/, crates/cli/_
  - _Writes: crates/cli/_
  - _Validation: Run command contract, stdin/file/parameter precedence, noninteractive input, remote trust, deeplink, stdout/stderr, machine-output, and exit-code tests_

- [ ] 14. Resolve and, if approved, implement one scheduled-recipe service
  - Define persisted job/state transitions and DST, missed-run, overlap, retry, restart, notification, and cleanup policies
  - Expose the same service through approved CLI, ACP, and desktop adapters
  - Add a constrained permission-checked agent tool only after the service and threat model are approved

  - _Requirements: 14.1, 14.2, 14.3, 14.4_
  - _Depends on: 5, 12, security-permissions/6, security-permissions/7, security-permissions/9_
  - _Reads: .agents/specs/goose-migration/recipe-system/requirements.md, .agents/specs/goose-migration/recipe-system/design.md, projects/goose/crates/goose/src/scheduler.rs, projects/goose/crates/goose/src/scheduler_trait.rs, projects/goose/crates/goose/src/agents/schedule_tool.rs, crates/scheduler/, crates/agent/, crates/agent_ui/_
  - _Writes: selected recipe/session scheduling owner, crates/agent/, crates/agent_ui/_
  - _Validation: Run persistence/restart, DST, missed-run, overlap, pause/run/cancel, generated-session linking, adapter parity, permission denial, secret redaction, and audit tests_

## Notes

- Recipe engine is a library; the CLI and desktop UI are consumers
- Built-in recipes ship inside the binary via `include_dir!`
- Scanner is a separate deployment artifact (Docker-based)
