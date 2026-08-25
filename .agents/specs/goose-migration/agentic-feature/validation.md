# Validation: Agentic compile-time feature boundary

Validated on Linux on 2026-08-25 with `zed` as the application feature owner.

## Product matrix

| Configuration | Build | Tests | Result |
| --- | --- | --- | --- |
| Default agentic product | `cargo build -p zed` | `cargo test -p zed` | Build passed; 80 passed, 0 failed, 1 ignored |
| Explicit agentic product | `cargo build -p zed --no-default-features --features agentic` | `cargo test -p zed --no-default-features --features agentic` | Build passed; 80 passed, 0 failed, 1 ignored |
| Non-agentic product | `cargo build -p zed --no-default-features` | `cargo test -p zed --no-default-features` | Build passed; 74 passed, 0 failed, 1 ignored |

The ignored test in each product is the existing cross-platform timing-sensitive `test_window_edit_state_restoring_enabled` test.

## Launch smoke check

The non-agentic product was launched with:

```bash
timeout 15s cargo run -q -p zed --no-default-features
```

The process remained alive without output or initialization failure until the bounded smoke check terminated it. Exit status 124 is the expected `timeout` result.

Supported interactive run commands are:

```bash
cargo run -p zed
cargo run -p zed --no-default-features --features agentic
cargo run -p zed --no-default-features
```

## Boundary and graph checks

`script/check-agentic-feature` passed. It verifies all of the following:

- the disabled normal dependency graph excludes the agent-only package denylist;
- the enabled graph contains the expected agentic packages;
- every participating crate has `agentic` enabled in the explicit-agentic inverse feature tree and absent in the disabled inverse feature tree;
- the disabled action registry omits agent namespaces and explicit agent/editor actions;
- disabled builds reject agent and skill-install URLs explicitly.

`cargo metadata --format-version 1 --no-deps` reported `zed`'s default feature as `agentic` and showed explicit forwarding to every participating crate. The following hygiene checks also passed:

```bash
cargo fmt --all -- --check
git diff --check
bash -n script/check-agentic-feature
python3 .agents/skills/feature-spec/scripts/validate_spec.py \
  .agents/specs/goose-migration/agentic-feature \
  --require-complete \
  --dialect canonical
```

The cross-pack link audit found the inherited feature-boundary contract in all 18 pre-existing Goose migration task plans. Each plan's diff adds only the two contract lines and does not change historical task IDs, state, dependencies, or evidence.
