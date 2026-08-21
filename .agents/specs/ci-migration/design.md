# Design: CI/CD Archived Workflow Migration

## Overview

Phase 1 reduces the active GitHub Actions surface to an explicit two-file allowlist and makes that state stable under workflow regeneration. Later phases restore archived workflows one group at a time; none of those restoration phases are part of the Phase 1 implementation.

## Measured starting state

| Inventory | Count |
|---|---:|
| Top-level `.github/workflows/*.yml` files | 41 |
| Retained core workflows | 2 |
| Non-core workflows to archive | 39 |
| Generated Zed workflows | 20 |
| Generated retained core workflows | 2 |
| Generated non-core workflows to archive | 18 |
| Hand-written non-core workflows to archive | 21 |

The retained core workflows are `release.yml` and `run_tests.yml`. At the start of Phase 1, all 39 non-core files are top-level and active; the archive contains only its README.

## Architecture and decisions

### Explicit active allowlist

Phase 1 defines the active Zed workflow allowlist as exactly `release.yml` and `run_tests.yml`. The migration is derived from a filesystem inventory, not a historical count or a curated subset: every other top-level `.yml` file is archived.

### History-preserving archive

Each non-core workflow is moved with `git mv` from `.github/workflows/<name>.yml` to `.github/workflows/archive/<name>.yml`. Workflow contents are not edited during the move. GitHub Actions discovers workflows only directly under `.github/workflows/`, so archived files remain version-controlled but inactive.

The post-Phase 1 layout is:

```text
.github/workflows/
├── release.yml
├── run_tests.yml
└── archive/
    ├── README.md
    └── 39 archived .yml files
```

The README inventory is checked against the directory contents and must list each archived filename exactly once.

### Generator policy

`tooling/xtask/src/tasks/workflows.rs` remains the source of truth for generated workflow definitions. `ARCHIVED_ZED_WORKFLOWS` contains all 18 generated non-core Zed workflow names. `WorkflowFile::active_output_path` returns no active path for those names, and generation therefore writes only the two core Zed workflows. The cleanup and write paths do not traverse `.github/workflows/archive/`, so archived files are not modified or removed.

The focused test derives active output paths from the same workflow source inventory used by generation. It asserts that the complete generated Zed set partitions into:

- active: `release.yml`, `run_tests.yml`;
- archived: every name in `ARCHIVED_ZED_WORKFLOWS`.

This makes a newly added generated Zed workflow fail the policy test until it is deliberately assigned to one side of the partition.

<!-- impl: tooling/xtask/src/tasks/workflows.rs#ARCHIVED_ZED_WORKFLOWS -->
<!-- impl: tooling/xtask/src/tasks/workflows.rs#WorkflowFile::active_output_path -->
<!-- impl: tooling/xtask/src/tasks/workflows.rs#zed_workflow_generation_matches_archive_policy -->

### Later restoration

A later restoration moves an archived workflow back to the top level only after its task is authorized. If it is generated, the same change removes its name from `ARCHIVED_ZED_WORKFLOWS`; otherwise `cargo xtask workflows` would remove or omit it. `run_bundling.yml` remains archived after Phase 1, and Task 3 is the first unstarted restoration task.

## Archive groups and dependencies

| Group | Workflows | Restoration consideration |
|---|---|---|
| Bundling | `run_bundling.yml` | Generated; remove from archive policy when restored. |
| Nightly release | `release_nightly.yml` | Generated; validate release secrets and notifications. |
| Post-release and docs | `after_release.yml`, `deploy_docs.yml`, `deploy_nightly_docs.yml`, `docs_suggestions.yml` | Restore reusable docs workflow before callers. |
| Collaboration | `deploy_collab.yml` | Validate registry and Kubernetes credentials. |
| Autofix, Nix, compliance | `autofix_pr.yml`, `nix_build.yml`, `compliance_check.yml` | Generated; validate tokens and caches. |
| Extensions | `extension_tests.yml`, `extension_bump.yml`, `extension_auto_bump.yml`, `publish_extension_cli.yml`, `extension_workflow_rollout.yml` | Generated group with cross-workflow and core-CI integration. |
| Review and release utilities | `danger.yml`, `pr_issue_labeler.yml`, `cherry_pick.yml`, `bump_zed_version.yml`, `bump_patch_version.yml`, `bump_collab_staging.yml` | Generated files must leave the archive policy when restored. |
| Community | all community, duplicate, ranking, triage, stale, congratulation, and notifier workflows | Restore only after event and permission review. |
| Slack | `hotfix-review-monitor.yml`, `slack_notify_community_automation_failure.yml`, `slack_notify_first_responders.yml`, `slack_notify_label_created.yml` | Validate destinations and secrets before activation. |

## Correctness properties

### Archive completeness

Every pre-migration top-level `.yml` file is either one of the two core workflows or appears with identical content under `.github/workflows/archive/` after Phase 1.

### Active-surface closure

The set of top-level `.github/workflows/*.yml` files equals the active allowlist before and after `cargo xtask workflows`.

### Archive immutability under generation

Running `cargo xtask workflows` does not change the archived workflow tree. This is checked through working-tree/index comparisons around generation.

### Restoration fidelity

When a later task restores a workflow, the archived content is the starting implementation and any required generator, dependency, secret, or permission update is made in the same restoration phase.

## Requirements traceability

| Acceptance criteria | Design/implementation surface | Verification |
|---|---|---|
| 1.1, 1.2 | Measured inventory and explicit active allowlist | Filesystem inventory check |
| 1.3, 1.5 | History-preserving archive and inactive archive location | Staged rename and active-set checks |
| 1.4 | Exact README inventory | Directory-to-README comparison |
| 2.1, 2.3, 2.4 | `ARCHIVED_ZED_WORKFLOWS`, active output path, focused partition test | Focused xtask test and active-set check |
| 2.2 | Generator cleanup/write scope | Archive diff before and after generation |
| 3.1, 3.2, 3.3, 3.4 | Later Task 3 bundling restoration | Task 3 workflow inspection |
| 4.1, 4.2, 4.3, 4.4 | Later nightly release restoration | Task 4 workflow inspection |
| 5.1, 5.2, 5.3, 5.4, 5.5, 5.6 | Later post-release restoration | Task 5 workflow inspection |
| 6.1, 6.2, 6.3 | Later collaboration deployment restoration | Task 6 workflow inspection |
| 7.1, 7.2, 7.3, 7.4 | Later documentation restoration | Tasks 5, 7, and 22 workflow inspection |
| 8.1, 8.2 | Later autofix restoration | Task 8 workflow inspection |
| 9.1, 9.2 | Later Nix restoration | Task 9 workflow inspection |
| 10.1, 10.2, 10.3 | Later compliance restoration | Task 10 workflow inspection |
| 11.1, 11.2, 11.3, 11.4 | Later extension restoration | Task 11 workflow inspection |
| 12.1, 12.2 | Later Danger restoration | Task 12 workflow inspection |
| 13.1, 13.2 | Later labeling restoration | Task 13 workflow inspection |
| 14.1 | Later cherry-pick restoration | Task 14 workflow inspection |
| 15.1 | Later version-bump restoration | Task 15 workflow inspection |
| 16.1, 16.2, 16.3, 16.4, 16.5, 16.6 | Later community restoration tasks | Tasks 16 through 20 workflow inspection |
| 17.1, 17.2, 17.3 | Later Slack restoration | Task 21 workflow inspection |

## Validation strategy

Phase 1 validation runs in this order:

1. Run the spec validator before and after edits.
2. Run `cargo test -p xtask zed_workflow_generation_matches_archive_policy -- --nocapture`.
3. Confirm only `release.yml` and `run_tests.yml` are top-level `.yml` workflows.
4. Confirm the archive contains 39 `.yml` files and the README inventory matches them exactly.
5. Confirm the 39 moves preserve content and history.
6. Run `cargo xtask workflows`.
7. Repeat active-surface and archive-completeness checks and confirm no archived file changed.
8. Run `cargo fmt --check --package xtask` and `git diff --check`.

External triggers, secrets, deployments, and workflow restoration are deferred to later phases.
