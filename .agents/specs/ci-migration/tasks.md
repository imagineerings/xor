# Implementation Plan: CI/CD Archived Workflow Migration

## Pre-Migration Setup (P0)

- [x] Create `.github/workflows/archive/README.md` with the exact archive inventory, active allowlist, generator policy, and restoration procedure.
  - _Evidence: The README lists all 39 archived workflows exactly once, names only `release.yml` and `run_tests.yml` as active, documents the 18-name generated archive policy, and requires `git mv` for later restoration._

## Phase 1: Archive non-core workflows and make regeneration safe (P0)

- [x] 1. Archive the measured non-core workflow inventory and enforce the generator policy.
  - [x] 1.1. Measure and record the actual workflow inventory.
    - _Requirements: 1.1_
    - _Depends on: none_
    - _Reads: `.github/workflows/*.yml`, `tooling/xtask/src/tasks/workflows.rs`_
    - _Writes: `.agents/specs/ci-migration/requirements.md`, `.agents/specs/ci-migration/design.md`, `.agents/specs/ci-migration/tasks.md`_
    - _Validation: Count top-level `.yml` files, retained core files, generated files, and non-core files from the checkout._
    - _Evidence: The measured baseline was 41 top-level `.yml` files: 2 retained core workflows and 39 non-core workflows. The generator inventory contained 20 Zed workflows, split into 2 active core and 18 archived generated workflows; the remaining 21 archived workflows are hand-written._
  - [x] 1.2. Move every non-core top-level workflow into the archive with `git mv` and make the README inventory exact.
    - _Requirements: 1.2, 1.3, 1.4, 1.5_
    - _Depends on: 1.1_
    - _Reads: `.github/workflows/*.yml`, `.github/workflows/archive/README.md`_
    - _Writes: `.github/workflows/*.yml`, `.github/workflows/archive/*.yml`, `.github/workflows/archive/README.md`_
    - _Validation: Confirm the active set is exactly `release.yml` and `run_tests.yml`; confirm 39 archived `.yml` files; compare the README inventory to the directory; inspect staged 100% renames._
    - _Evidence: `git mv` produced 39 staged 100% renames with zero content changes. The active inventory contains exactly 2 files, the archive contains 39 `.yml` files, and a sorted README-to-directory comparison has no differences._
  - [x] 1.3. Exclude every generated archived Zed workflow and test the complete active/archive partition.
    - _Requirements: 2.1, 2.3, 2.4_
    - _Depends on: 1.2_
    - _Reads: `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/*.yml`_
    - _Writes: `tooling/xtask/src/tasks/workflows.rs`_
    - _Validation: `cargo test -p xtask zed_workflow_generation_matches_archive_policy -- --nocapture`_
    - _Evidence: `ARCHIVED_ZED_WORKFLOWS` contains all 18 generated archived Zed workflows, including `run_bundling`; the focused policy test passed with 1 test and asserts the active two-file allowlist, complete partition, and archived-file existence._
  - [x] 1.4. Validate deterministic generation, archive immutability, formatting, and the completed spec pack.
    - _Requirements: 1.2, 1.3, 2.1, 2.2, 2.4_
    - _Depends on: 1.3_
    - _Reads: `.agents/specs/ci-migration/requirements.md`, `.agents/specs/ci-migration/design.md`, `.agents/specs/ci-migration/tasks.md`, `.github/workflows/`, `tooling/xtask/src/tasks/workflows.rs`_
    - _Writes: `.agents/specs/ci-migration/tasks.md`_
    - _Validation: Run the spec validator; `cargo xtask workflows`; repeat active and archive completeness checks; confirm archived workflow diffs are empty; `cargo fmt --check --package xtask`; `git diff --check`._
    - _Evidence: The corrected spec validator passed with 56 acceptance criteria and 27 numeric tasks. `cargo xtask workflows` passed and left only `release.yml` and `run_tests.yml` active; the archive remained at 39 files with an unchanged README checksum and no archived working-tree diff. The focused test, archive-policy comparison, formatting check, and diff check passed._

The optional post-generation CI fallback is not needed because Task 1 enforces the policy in the generator and its focused test.

## Phase 2: Restore on-demand bundling (P0, not started)

- [ ] 3. Restore `run_bundling.yml` only after Phase 1 is complete and restoration is authorized.
  - [ ] 3.1. Move `archive/run_bundling.yml` to the active directory and remove `run_bundling` from the archived generator policy.
    - _Requirements: 3.1, 3.2, 3.3, 3.4_
    - _Depends on: 1.4_
    - _Reads: `.github/workflows/archive/run_bundling.yml`, `tooling/xtask/src/tasks/workflows.rs`_
    - _Writes: `.github/workflows/run_bundling.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
    - _Validation: Run the focused xtask policy test and `cargo xtask workflows`; confirm `run_bundling.yml` remains active after regeneration._
  - [ ] 3.2. Verify the restored workflow triggers, bundle matrix, artifact uploads, and label-removal behavior.
    - _Requirements: 3.1, 3.2, 3.3, 3.4_
    - _Depends on: 3.1_
    - _Reads: `.github/workflows/run_bundling.yml`_
    - _Writes: none_
    - _Validation: Inspect pull-request event types, label conditions, platform/architecture jobs, and artifact upload steps without triggering external actions._

## Phase 3: Restore nightly releases (P0)

- [ ] 4. Restore and validate `release_nightly.yml`.
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/release_nightly.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/release_nightly.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect schedule, tag decision, test/build/upload, tag update, and Slack failure paths; verify required secrets._

## Phase 4: Restore post-release automation (P0)

- [ ] 5. Restore `deploy_docs.yml` and `after_release.yml` as a dependency-ordered group.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 7.3_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/deploy_docs.yml`, `.github/workflows/archive/after_release.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/deploy_docs.yml`, `.github/workflows/after_release.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect release trigger, reusable-workflow reference, release-page refresh, docs channels, Discord, Winget, Sentry, Slack failure handling, and secrets._

## Phase 5: Restore collaboration deployment (P1)

- [ ] 6. Restore and validate `deploy_collab.yml`.
  - _Requirements: 6.1, 6.2, 6.3_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/deploy_collab.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/deploy_collab.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect production tag, checks, image publication, Kubernetes deployment, and required credentials._

## Phase 6: Restore documentation automation (P1)

- [ ] 7. Restore `deploy_nightly_docs.yml` after the reusable docs workflow and validate `docs_suggestions.yml` separately.
  - _Requirements: 7.1, 7.2, 7.4_
  - _Depends on: 5_
  - _Reads: `.github/workflows/archive/deploy_nightly_docs.yml`, `.github/workflows/archive/docs_suggestions.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/deploy_nightly_docs.yml`, `.github/workflows/docs_suggestions.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect main-branch trigger, mdBook build, Cloudflare nightly channel, reusable-workflow reference, and suggestion scope._

## Phase 7: Restore pull-request autofix (P1)

- [ ] 8. Restore and validate `autofix_pr.yml`.
  - _Requirements: 8.1, 8.2_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/autofix_pr.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/autofix_pr.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect dispatch input, fix tools, change detection, bot commit/push path, permissions, and token secret._

## Phase 8: Restore Nix builds (P1)

- [ ] 9. Restore and validate `nix_build.yml`.
  - _Requirements: 9.1, 9.2_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/nix_build.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/nix_build.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect label conditions, Linux/macOS builds, Cachix publication, permissions, and credentials._

## Phase 9: Restore compliance checks (P1)

- [ ] 10. Restore and validate `compliance_check.yml`.
  - _Requirements: 10.1, 10.2, 10.3_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/compliance_check.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/compliance_check.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect schedule, preview-tag lookup, report artifact, Slack failure path, and secrets._

## Phase 10: Restore extension automation (P1)

- [ ] 11. Restore the five extension workflows and their core-CI integration as one dependency group.
  - _Requirements: 11.1, 11.2, 11.3, 11.4_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/extension_tests.yml`, `.github/workflows/archive/extension_bump.yml`, `.github/workflows/archive/extension_auto_bump.yml`, `.github/workflows/archive/publish_extension_cli.yml`, `.github/workflows/archive/extension_workflow_rollout.yml`, `.github/workflows/run_tests.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/extension_tests.yml`, `.github/workflows/extension_bump.yml`, `.github/workflows/extension_auto_bump.yml`, `.github/workflows/publish_extension_cli.yml`, `.github/workflows/extension_workflow_rollout.yml`, `.github/workflows/run_tests.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation and core tests; inspect extension change detection, validation, version checks, CLI publication, rollout references, release pull requests, permissions, and secrets._

## Phase 11: Restore Danger checks (P2)

- [ ] 12. Restore and validate `danger.yml`.
  - _Requirements: 12.1, 12.2_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/danger.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/danger.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect pull-request and merge-queue triggers and Danger execution._

## Phase 12: Restore pull-request and issue labeling (P2)

- [ ] 13. Restore and validate `pr_issue_labeler.yml`.
  - _Requirements: 13.1, 13.2_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/pr_issue_labeler.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/pr_issue_labeler.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect pull-request and issue triggers, label rules, permissions, and community-bot token._

## Phase 13: Restore cherry-pick automation (P2)

- [ ] 14. Restore and validate `cherry_pick.yml`.
  - _Requirements: 14.1_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/cherry_pick.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/cherry_pick.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect dispatch inputs, cherry-pick command, target push, permissions, and token._

## Phase 14: Restore automated version bumping (P2)

- [ ] 15. Restore and validate `bump_zed_version.yml`, `bump_patch_version.yml`, and `bump_collab_staging.yml`.
  - _Requirements: 15.1_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/bump_zed_version.yml`, `.github/workflows/archive/bump_patch_version.yml`, `.github/workflows/archive/bump_collab_staging.yml`, `tooling/xtask/src/tasks/workflows.rs`_
  - _Writes: `.github/workflows/bump_zed_version.yml`, `.github/workflows/bump_patch_version.yml`, `.github/workflows/bump_collab_staging.yml`, `tooling/xtask/src/tasks/workflows.rs`, `.github/workflows/archive/README.md`_
  - _Validation: Run generator validation; inspect dispatch inputs, version calculation, branches/tags, permissions, and app tokens._

## Phase 15: Restore community management (P3)

- [ ] 16. Restore stale issue and pull-request management.
  - _Requirements: 16.1_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/community_close_stale_issues.yml`, `.github/workflows/archive/stale-pr-reminder.yml`_
  - _Writes: `.github/workflows/community_close_stale_issues.yml`, `.github/workflows/stale-pr-reminder.yml`, `.github/workflows/archive/README.md`_
  - _Validation: Inspect schedules, stale thresholds, warning/close behavior, permissions, and notification targets._

- [ ] 17. Restore community pull-request project synchronization.
  - _Requirements: 16.3_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/community_pr_board.yml`, `.github/workflows/archive/community_pr_board_refresh.yml`_
  - _Writes: `.github/workflows/community_pr_board.yml`, `.github/workflows/community_pr_board_refresh.yml`, `.github/workflows/archive/README.md`_
  - _Validation: Inspect pull-request and refresh triggers, project operations, permissions, and tokens._

- [ ] 18. Restore all-time and weekly issue-ranking maintenance.
  - _Requirements: 16.5_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/community_update_all_top_ranking_issues.yml`, `.github/workflows/archive/community_update_weekly_top_ranking_issues.yml`_
  - _Writes: `.github/workflows/community_update_all_top_ranking_issues.yml`, `.github/workflows/community_update_weekly_top_ranking_issues.yml`, `.github/workflows/archive/README.md`_
  - _Validation: Inspect schedules, ranking scopes, mutations, permissions, and tokens._

- [ ] 19. Restore duplicate-issue management.
  - _Requirements: 16.2_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/comment_on_potential_duplicate_issues.yml`, `.github/workflows/archive/track_duplicate_bot_effectiveness.yml`, `.github/workflows/archive/update_duplicate_magnets.yml`_
  - _Writes: `.github/workflows/comment_on_potential_duplicate_issues.yml`, `.github/workflows/track_duplicate_bot_effectiveness.yml`, `.github/workflows/update_duplicate_magnets.yml`, `.github/workflows/archive/README.md`_
  - _Validation: Inspect duplicate detection, comments, effectiveness tracking, magnet maintenance, permissions, and tokens._

- [ ] 20. Restore the remaining community event workflows.
  - _Requirements: 16.4, 16.6_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/good_first_issue_notifier.yml`, `.github/workflows/archive/catch_blank_issues.yml`, `.github/workflows/archive/congrats.yml`, `.github/workflows/archive/add_commented_closed_issue_to_project.yml`, `.github/workflows/archive/triage_project_sync.yml`_
  - _Writes: `.github/workflows/good_first_issue_notifier.yml`, `.github/workflows/catch_blank_issues.yml`, `.github/workflows/congrats.yml`, `.github/workflows/add_commented_closed_issue_to_project.yml`, `.github/workflows/triage_project_sync.yml`, `.github/workflows/archive/README.md`_
  - _Validation: Inspect each event scope, action, permission, project mutation, and notification target._

## Phase 16: Restore Slack notifications (P3)

- [ ] 21. Restore all four Slack notification workflows after destination and secret review.
  - _Requirements: 17.1, 17.2, 17.3_
  - _Depends on: 1.4_
  - _Reads: `.github/workflows/archive/hotfix-review-monitor.yml`, `.github/workflows/archive/slack_notify_community_automation_failure.yml`, `.github/workflows/archive/slack_notify_first_responders.yml`, `.github/workflows/archive/slack_notify_label_created.yml`_
  - _Writes: `.github/workflows/hotfix-review-monitor.yml`, `.github/workflows/slack_notify_community_automation_failure.yml`, `.github/workflows/slack_notify_first_responders.yml`, `.github/workflows/slack_notify_label_created.yml`, `.github/workflows/archive/README.md`_
  - _Validation: Inspect hotfix review detection, failure notifications, label/responder triggers, channel destinations, permissions, and webhook secrets._

## Phase 17: Documentation suggestions (P2)

- [ ] 22. This identifier is retained for plan history; documentation suggestions are implemented as part of dependency-aware Task 7.
  - _Requirements: 7.4_
  - _Depends on: 7_
  - _Reads: `.github/workflows/docs_suggestions.yml`_
  - _Writes: none_
  - _Validation: Confirm Task 7 completed the isolated suggestions workflow restoration and did not activate unrelated workflows._

## Execution rules

- Every restoration move uses `git mv`.
- A generated workflow is removed from `ARCHIVED_ZED_WORKFLOWS` in the same task that restores it.
- Every later phase validates dependencies, permissions, and secrets before activation.
- Phase 1 stops before Task 3; no archived workflow is restored or activated.
