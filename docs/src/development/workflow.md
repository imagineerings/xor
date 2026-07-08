---
title: Development Workflow
description: Work effectively in Sim's Rust workspace with focused branches, tests, and reviewable commits.
---

# Development Workflow

Keep changes small, validated, and easy to review. The repository is large, so focused branches and targeted tests matter.

## Branches

Use a short branch name that describes the change. Keep unrelated refactors out of feature branches unless they are required by the task.

## Editing

Prefer existing crate patterns over new abstractions. Read the module you are changing and nearby tests before editing.

## Testing

Pick validation based on risk:

- Single helper or parser: focused unit tests.
- Shared API: crate tests and affected integration tests.
- GPUI behavior: use GPUI test helpers and deterministic scheduler reproduction when needed.
- Broad workspace change: `./script/clippy` and relevant `cargo nextest` targets.

## Review

Before opening a pull request:

1. Review your own diff.
2. Run formatting and relevant validation.
3. Write a PR summary that explains behavior and risk.
4. Include release notes or `N/A` as appropriate.

## Landing

Wait for required checks, address review feedback in follow-up commits, and squash merge once the branch is ready.
