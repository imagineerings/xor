# Implementation Plan: CI/CD Archived Workflow Migration

## Overview

This plan covers restoring archived GitHub Actions workflows in priority order. Workflows are grouped for parallel restoration when they have no cross-dependencies. Each task corresponds to either moving a file from `archive/` back to `.github/workflows/` or making configuration changes.

### Priority Legend

- **P0**: Needed for release process (blocking)
- **P1**: Needed for development workflow
- **P2**: Nice to have
- **P3**: Community/operational (lowest urgency)

## Pre-Migration Setup

- [x] P0: Create `.github/workflows/archive/README.md` documenting the archive structure and migration procedure
  - _Writes: `.github/workflows/archive/README.md`_

## Tasks

### Phase 1: XTask Regeneration Safety (P0)

- [x] 1. Update xtask workflow generator to skip archived workflows
  - [ ] 1.1 Locate the workflow generation code in `tooling/xtask/src/tasks/workflows/`
  - [ ] 1.2 Add an exclusion list of archived workflow names
  - [ ] 1.3 Modify the output writer to skip excluded workflows (or redirect to `archive/`)
  - [ ] 1.4 Run `cargo xtask workflows` and verify archived files are not written to `.github/workflows/`
  - _Requirements: R17_
  - _Reads: `tooling/xtask/src/tasks/workflows/`_

- [ ] 2. Create post-generation safety check in CI (optional, if xtask cannot be modified)
  - [ ] 2.1 Add a script that checks for unexpected workflow regeneration
  - [ ] 2.2 Run it as part of `run_tests.yml` check_scripts job
  - _Requirements: R17_
  - _Writes: `script/check-archived-workflows`, modification to `run_tests.yml`_

### Phase 2: On-Demand Bundling (P0)

- [x] 3. Restore `run_bundling.yml`
  - [ ] 3.1 Move `archive/run_bundling.yml` to `.github/workflows/run_bundling.yml`
  - [ ] 3.2 Verify no path changes needed (the workflow is self-contained)
  - _Requirements: R1_
  - _Reads: `.github/workflows/archive/run_bundling.yml`_

### Phase 3: Nightly Releases (P0)

- [ ] 4. Restore `release_nightly.yml`
  - [ ] 4.1 Move `archive/release_nightly.yml` to `.github/workflows/release_nightly.yml`
  - [ ] 4.2 Verify secrets are configured (DigitalOcean Spaces, Slack webhook, Sentry, Cachix, GitHub app tokens)
  - _Requirements: R2_
  - _Reads: `.github/workflows/archive/release_nightly.yml`_

### Phase 4: Post-Release Automation (P0)

- [ ] 5. Restore post-release workflows
  - [ ] 5.1 Move `archive/after_release.yml` to `.github/workflows/after_release.yml`
  - [ ] 5.2 Move `archive/deploy_docs.yml` to `.github/workflows/deploy_docs.yml`
  - [ ] 5.3 Verify the composite reference `uses: simtropolis/sim/.github/workflows/deploy_docs.yml@...` in `after_release.yml` points to the restored file (or update the commit SHA)
  - [ ] 5.4 Verify secrets are configured (Discord webhook, Winget token, Sentry, Vercel, Cloudflare, Amplitude)
  - _Requirements: R3, R5_
  - _Reads: `.github/workflows/archive/after_release.yml`, `.github/workflows/archive/deploy_docs.yml`_

### Phase 5: Collab Server Deployment (P1)

- [ ] 6. Restore `deploy_collab.yml`
  - [ ] 6.1 Move `archive/deploy_collab.yml` to `.github/workflows/deploy_collab.yml`
  - [ ] 6.2 Verify secrets are configured (DigitalOcean token, Kubernetes cluster name)
  - _Requirements: R4_
  - _Reads: `.github/workflows/archive/deploy_collab.yml`_

### Phase 6: Nightly Docs Deployment (P1)

- [ ] 7. Restore `deploy_nightly_docs.yml`
  - [ ] 7.1 Move `archive/deploy_nightly_docs.yml` to `.github/workflows/deploy_nightly_docs.yml`
  - [ ] 7.2 Verify the composite reference `uses: simtropolis/sim/.github/workflows/deploy_docs.yml@...` points to the restored file
  - _Requirements: R5_
  - _Reads: `.github/workflows/archive/deploy_nightly_docs.yml`_

### Phase 7: Auto-Fix PRs (P1)

- [ ] 8. Restore `autofix_pr.yml`
  - [ ] 8.1 Move `archive/autofix_pr.yml` to `.github/workflows/autofix_pr.yml`
  - [ ] 8.2 Verify GitHub app token secrets are configured
  - _Requirements: R6_
  - _Reads: `.github/workflows/archive/autofix_pr.yml`_

### Phase 8: Nix Build (P1)

- [ ] 9. Restore `nix_build.yml`
  - [ ] 9.1 Move `archive/nix_build.yml` to `.github/workflows/nix_build.yml`
  - [ ] 9.2 Verify Cachix and other secrets are configured
  - _Requirements: R7_
  - _Reads: `.github/workflows/archive/nix_build.yml`_

### Phase 9: Compliance Checks (P1)

- [ ] 10. Restore `compliance_check.yml`
  - [ ] 10.1 Move `archive/compliance_check.yml` to `.github/workflows/compliance_check.yml`
  - [ ] 10.2 Verify GitHub app token secrets are configured
  - _Requirements: R8_
  - _Reads: `.github/workflows/archive/compliance_check.yml`_

### Phase 10: Extension CI (P1)

- [ ] 11. Restore extension-related workflows
  - [ ] 11.1 Move `archive/extension_tests.yml` to `.github/workflows/extension_tests.yml`
  - [ ] 11.2 Re-add `extension_tests` job back to `run_tests.yml`:
    - Add the `extension_tests` job block (from `archive/extension_tests.yml` or git history)
    - Add `extension_tests` to `tests_pass` needs list
    - Add extension_tests check result to `tests_pass` step
    - Add `RESULT_EXTENSION_TESTS` env var to `tests_pass`
  - [ ] 11.3 Move `archive/extension_bump.yml` to `.github/workflows/extension_bump.yml`
  - [ ] 11.4 Move `archive/extension_auto_bump.yml` to `.github/workflows/extension_auto_bump.yml`
  - [ ] 11.5 Move `archive/publish_extension_cli.yml` to `.github/workflows/publish_extension_cli.yml`
  - [ ] 11.6 Move `archive/extension_workflow_rollout.yml` to `.github/workflows/extension_workflow_rollout.yml`
  - [ ] 11.7 Verify secrets are configured (GitHub app tokens, DigitalOcean Spaces)
  - _Requirements: R9_
  - _Reads: `.github/workflows/archive/extension_tests.yml`, `.github/workflows/run_tests.yml`_

### Phase 11: Danger Checks (P2)

- [ ] 12. Restore `danger.yml`
  - [ ] 12.1 Move `archive/danger.yml` to `.github/workflows/danger.yml`
  - [ ] 12.2 No additional configuration needed (uses danger-proxy)
  - _Requirements: R10_
  - _Reads: `.github/workflows/archive/danger.yml`_

### Phase 12: PR/Issue Labeler (P2)

- [ ] 13. Restore `pr_issue_labeler.yml`
  - [ ] 13.1 Move `archive/pr_issue_labeler.yml` to `.github/workflows/pr_issue_labeler.yml`
  - [ ] 13.2 Verify GitHub app token secrets are configured (community bot)
  - _Requirements: R11_
  - _Reads: `.github/workflows/archive/pr_issue_labeler.yml`_

### Phase 13: Cherry-Pick (P2)

- [ ] 14. Restore `cherry_pick.yml`
  - [ ] 14.1 Move `archive/cherry_pick.yml` to `.github/workflows/cherry_pick.yml`
  - [ ] 14.2 Verify GitHub app token secrets are configured
  - _Requirements: R12_
  - _Reads: `.github/workflows/archive/cherry_pick.yml`_

### Phase 14: Version Bumping (P2)

- [ ] 15. Restore version bumping workflows
  - [ ] 15.1 Move `archive/bump_sim_version.yml` to `.github/workflows/bump_sim_version.yml`
  - [ ] 15.2 Move `archive/bump_patch_version.yml` to `.github/workflows/bump_patch_version.yml`
  - [ ] 15.3 Move `archive/bump_collab_staging.yml` to `.github/workflows/bump_collab_staging.yml`
  - [ ] 15.4 Verify GitHub app token secrets are configured
  - _Requirements: R13_
  - _Reads: `.github/workflows/archive/bump_sim_version.yml`_

### Phase 15: Community Management (P3)

- [ ] 16. Restore stale issue/pr management
  - [ ] 16.1 Move `archive/community_close_stale_issues.yml` to `.github/workflows/community_close_stale_issues.yml`
  - [ ] 16.2 Move `archive/stale-pr-reminder.yml` to `.github/workflows/stale-pr-reminder.yml`
  - _Requirements: R14_

- [ ] 17. Restore community PR board
  - [ ] 17.1 Move `archive/community_pr_board.yml` to `.github/workflows/community_pr_board.yml`
  - [ ] 17.2 Move `archive/community_pr_board_refresh.yml` to `.github/workflows/community_pr_board_refresh.yml`
  - _Requirements: R14_

- [ ] 18. Restore top ranking issue updates
  - [ ] 18.1 Move `archive/community_update_all_top_ranking_issues.yml` to `.github/workflows/community_update_all_top_ranking_issues.yml`
  - [ ] 18.2 Move `archive/community_update_weekly_top_ranking_issues.yml` to `.github/workflows/community_update_weekly_top_ranking_issues.yml`
  - _Requirements: R14_

- [ ] 19. Restore duplicate issue management
  - [ ] 19.1 Move `archive/comment_on_potential_duplicate_issues.yml` to `.github/workflows/comment_on_potential_duplicate_issues.yml`
  - [ ] 19.2 Move `archive/track_duplicate_bot_effectiveness.yml` to `.github/workflows/track_duplicate_bot_effectiveness.yml`
  - [ ] 19.3 Move `archive/update_duplicate_magnets.yml` to `.github/workflows/update_duplicate_magnets.yml`
  - _Requirements: R14_

- [ ] 20. Restore other community workflows
  - [ ] 20.1 Move `archive/catch_blank_issues.yml` to `.github/workflows/catch_blank_issues.yml`
  - [ ] 20.2 Move `archive/good_first_issue_notifier.yml` to `.github/workflows/good_first_issue_notifier.yml`
  - [ ] 20.3 Move `archive/congrats.yml` to `.github/workflows/congrats.yml`
  - [ ] 20.4 Move `archive/add_commented_closed_issue_to_project.yml` to `.github/workflows/add_commented_closed_issue_to_project.yml`
  - [ ] 20.5 Move `archive/triage_project_sync.yml` to `.github/workflows/triage_project_sync.yml`
  - _Requirements: R14_

### Phase 16: Slack Notifications (P3)

- [ ] 21. Restore Slack notification workflows
  - [ ] 21.1 Move `archive/slack_notify_first_responders.yml` to `.github/workflows/slack_notify_first_responders.yml`
  - [ ] 21.2 Move `archive/slack_notify_label_created.yml` to `.github/workflows/slack_notify_label_created.yml`
  - [ ] 21.3 Move `archive/hotfix-review-monitor.yml` to `.github/workflows/hotfix-review-monitor.yml`
  - [ ] 21.4 Verify Slack webhook secrets are configured
  - _Requirements: R15_

### Phase 17: Docs Suggestions (P2)

- [ ] 22. Restore `docs_suggestions.yml`
  - [ ] 22.1 Move `archive/docs_suggestions.yml` to `.github/workflows/docs_suggestions.yml`
  - _Requirements: R5_
  - _Reads: `.github/workflows/archive/docs_suggestions.yml`_

## Notes

- All "move" tasks use `git mv` to preserve file history: `git mv .github/workflows/archive/<file>.yml .github/workflows/<file>.yml`
- After restoring any workflow, verify it appears in the GitHub Actions UI and can be triggered
- For workflows with auto-generated headers, ensure the xtask exclusion (Task 1) is completed first, or re-archive the regenerated file after running `cargo xtask workflows`
- The `extension_tests.yml` re-integration (Task 11.2) requires careful editing of `run_tests.yml` to add back the job definition and the `tests_pass` references
