# Development Scripts

This directory contains small developer-facing helpers migrated for the Goose
workflow. They complement the long-standing top-level `script/` directory by
grouping agent migration and validation entrypoints.

## Windows build

```sh
pwsh scripts/windows-build.ps1 -Architecture x86_64
```

Wraps `script/bundle-windows.ps1` and keeps the migration-facing command stable.

## OpenAPI schema validation

```sh
node scripts/validate-openapi-schema.js path/to/openapi.json
node scripts/validate-openapi-schema.js http://127.0.0.1:8080/openapi.json
```

Validates required OpenAPI document shape, operation objects, parameters,
request bodies, responses, and local `$ref` targets.

## Diagnostics viewer

```sh
node scripts/diagnostics-viewer.js diagnostics.jsonl
```

Reads JSON, JSONL, or plain-text diagnostics and prints a compact summary by
severity plus the first few entries.

## Database helper

```sh
scripts/database-helper.sh list-test
scripts/database-helper.sh drop-test --yes
scripts/database-helper.sh reset-dev --yes
```

Wraps existing database scripts with explicit confirmation for destructive
commands.

## MCP testing

```sh
scripts/test-mcp-servers.sh
```

Runs the in-tree MCP server package tests.

## Sub-agent and recipe testing

```sh
scripts/test-sub-agent-and-recipe.sh
```

Runs focused agent sub-agent and recipe crate tests.

## Pre-release checks

```sh
scripts/prerelease-check.sh quick
scripts/prerelease-check.sh full
```

Runs a migration-oriented pre-release checklist. `quick` avoids the heavier
workspace test pass; `full` adds broader checks.

## Compaction testing

```sh
scripts/test-compaction.sh
```

Runs focused tests for automatic compaction settings and agent thread compaction
behavior.
