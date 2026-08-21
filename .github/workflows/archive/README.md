# Archived GitHub Actions workflows

Workflows placed in this directory are intentionally inactive. GitHub Actions only loads workflow files directly under `.github/workflows/`; it does not load files from this subdirectory.

The migration retains only these active workflows:

- `../run_tests.yml` for pull request, push, and merge-queue validation.
- `../release.yml` for release builds and publication to GitHub Releases.

## Archived workflow inventory

### Release, deployment, and packaging

- `after_release.yml` — post-release docs, Discord, Winget, and Sentry automation.
- `bump_collab_staging.yml` — collaboration staging version updates.
- `bump_patch_version.yml` — patch release version updates.
- `bump_zed_version.yml` — preview and stable version updates.
- `compliance_check.yml` — scheduled release compliance reporting.
- `deploy_collab.yml` — collaboration server image publication and deployment.
- `deploy_docs.yml` — reusable documentation deployment.
- `deploy_nightly_docs.yml` — nightly documentation deployment.
- `nix_build.yml` — Nix builds and Cachix publication.
- `release_nightly.yml` — scheduled nightly releases.
- `run_bundling.yml` — label-triggered multi-platform bundle builds.

### Extension automation

- `extension_auto_bump.yml` — automatic extension version bump requests.
- `extension_bump.yml` — extension version bump and release automation.
- `extension_tests.yml` — extension validation workflow.
- `extension_workflow_rollout.yml` — shared workflow rollout to extension repositories.
- `publish_extension_cli.yml` — extension CLI publication and reference updates.

### Pull request and maintenance automation

- `autofix_pr.yml` — dispatched formatting and lint fixes for pull requests.
- `cherry_pick.yml` — dispatched cherry-picks to release branches.
- `danger.yml` — pull request quality checks.
- `docs_suggestions.yml` — documentation suggestions on pull requests.
- `pr_issue_labeler.yml` — contributor and author-type labeling.

### Community and triage automation

- `add_commented_closed_issue_to_project.yml` — project tracking for activity on closed issues.
- `catch_blank_issues.yml` — blank issue handling.
- `comment_on_potential_duplicate_issues.yml` — duplicate issue suggestions.
- `community_close_stale_issues.yml` — stale issue closure.
- `community_pr_board.yml` — community pull request project updates.
- `community_pr_board_refresh.yml` — community pull request board refreshes.
- `community_update_all_top_ranking_issues.yml` — full issue-ranking refreshes.
- `community_update_weekly_top_ranking_issues.yml` — weekly issue-ranking refreshes.
- `congrats.yml` — first-contribution messages.
- `good_first_issue_notifier.yml` — good-first-issue notifications.
- `stale-pr-reminder.yml` — stale pull request reminders.
- `track_duplicate_bot_effectiveness.yml` — duplicate-detection reporting.
- `triage_project_sync.yml` — triage project synchronization.
- `update_duplicate_magnets.yml` — duplicate-magnet maintenance.

### Notifications

- `hotfix-review-monitor.yml` — post-merge hotfix review reminders.
- `slack_notify_community_automation_failure.yml` — community automation failure notifications.
- `slack_notify_first_responders.yml` — first-responder notifications.
- `slack_notify_label_created.yml` — label creation notifications.

## Regenerating workflows

Run `cargo xtask workflows` from the repository root to regenerate active workflows. The generator excludes its archived Zed workflows, so it writes only `run_tests.yml` and `release.yml` to the active Zed workflow directory. It does not overwrite archived copies in this directory. Extension-repository workflow outputs under `extensions/workflows/` are unaffected.

The exclusion list is `ARCHIVED_ZED_WORKFLOWS` in `tooling/xtask/src/tasks/workflows.rs`. When an archived generated workflow is intentionally restored, remove its name from that list in the same change. Otherwise regeneration will remove the active generated file and skip recreating it.

## Restoring a workflow

1. Identify the restoration task in `.agents/specs/ci-migration/tasks.md` and review its requirements, dependencies, required secrets, and cross-workflow references.
2. Move the archived file into `.github/workflows/` with `git mv`.
3. For an auto-generated workflow, also remove its name from `ARCHIVED_ZED_WORKFLOWS`, run `cargo xtask workflows`, and confirm regeneration preserves the restored active file. Do not hand-edit generated YAML.
4. Restore dependent workflows together where required. In particular, `after_release.yml` and `deploy_nightly_docs.yml` depend on `deploy_docs.yml`, and extension automation has shared workflow dependencies.
5. Validate permissions, triggers, references, and required repository secrets before merging. Confirm the restored workflow appears in GitHub Actions only after the intended activation change is reviewed.

Do not restore an archived workflow merely to edit or inspect it. Changes can be reviewed in place while it remains inactive.
