# Archived GitHub Actions Workflows

This directory contains GitHub Actions workflows that have been **archived** as part of the CI/CD simplification. They are not executed by GitHub Actions (subdirectories of `.github/workflows/` are not scanned). The two retained core workflows are:

- **`.github/workflows/run_tests.yml`** — CI tests on pull request, push, and merge queue
- **`.github/workflows/release.yml`** — Build, bundle, and publish to GitHub Releases on tag push

## Why were these archived?

To reduce the CI surface to a bare minimum (build, test, publish). All auxiliary workflows — community management, release automation, extension CI, issue triage, deployment, and project maintenance — have been moved here for clarity and to reduce noise in the GitHub Actions UI.

## How to restore a workflow

1. Move the file from `archive/` to the parent directory:
   ```bash
   git mv .github/workflows/archive/<workflow>.yml .github/workflows/<workflow>.yml
   ```
2. If the workflow is auto-generated (has `# Generated from xtask::workflows::...` header), also update the xtask generation code in `tooling/xtask/src/tasks/workflows/` to add it back to the active set.
3. Commit and push.

## Cross-reference dependencies

Some archived workflows reference other archived workflows. Restore them as a group:

| Workflow | Depends On |
|----------|------------|
| `after_release.yml` | `deploy_docs.yml` |
| `deploy_nightly_docs.yml` | `deploy_docs.yml` |
| `extension_bump.yml` | `extension_tests.yml` |
| `extension_auto_bump.yml` | `extension_tests.yml` |

## Auto-generation caveat

Many of these workflows have the header:

```yaml
# Generated from xtask::workflows::<name>
# Rebuild with `cargo xtask workflows`.
```

Running `cargo xtask workflows` will regenerate these files into `.github/workflows/`. **Before restoring any auto-generated workflow**, ensure one of:

1. The xtask generation code has been updated to skip the archived exclusion list (see `tooling/xtask/src/tasks/workflows/`), OR
2. You re-archive the regenerated files after running `cargo xtask workflows`.

## Restoration priority

See `.agents/specs/ci-migration/tasks.md` for the full prioritized implementation plan. Summary:

| Priority | Workflows |
|----------|-----------|
| **P0** (Release blocking) | `run_bundling`, `release_nightly`, `after_release`, `deploy_docs` |
| **P1** (Development workflow) | `deploy_collab`, `deploy_nightly_docs`, `autofix_pr`, `nix_build`, `compliance_check`, extension CI |
| **P2** (Nice to have) | `danger`, `pr_issue_labeler`, `cherry_pick`, version bumping, `docs_suggestions` |
| **P3** (Community/operational) | Community management, Slack notifications |

## Complete list of archived workflows

### Release & Build

| File | Description |
|------|-------------|
| `run_bundling.yml` | On-demand multi-platform bundling via PR label `run-bundling` |
| `release_nightly.yml` | Scheduled nightly release builds (every 4 hours) |
| `after_release.yml` | Post-release tasks (Discord, Winget, docs, Sentry) |

### Deployment

| File | Description |
|------|-------------|
| `deploy_collab.yml` | Deploy collaboration server on `collab-production` tag |
| `deploy_docs.yml` | Build and deploy docs to Cloudflare Pages (reusable workflow) |
| `deploy_nightly_docs.yml` | Deploy nightly docs on push to `main` |

### Code Quality

| File | Description |
|------|-------------|
| `danger.yml` | Danger CI checks on PRs |
| `autofix_pr.yml` | Auto-fix PRs with clippy, fmt, prettier |
| `compliance_check.yml` | Weekly compliance checks (SOC2) |

### Extension Ecosystem

| File | Description |
|------|-------------|
| `extension_tests.yml` | Extension CI tests (called by `run_tests.yml`) |
| `extension_bump.yml` | Bump extension versions |
| `extension_auto_bump.yml` | Auto-bump extension versions |
| `publish_extension_cli.yml` | Build and publish the extension CLI tool |
| `extension_workflow_rollout.yml` | Roll out extension workflow changes |

### Infrastructure

| File | Description |
|------|-------------|
| `nix_build.yml` | Nix build via PR label `run-nix` |
| `cherry_pick.yml` | Cherry-pick commits into release branches |
| `bump_baymax_version.yml` | Bump Baymax version (main/preview/stable) |
| `bump_patch_version.yml` | Bump patch version |
| `bump_collab_staging.yml` | Bump collab staging version |

### Community Management

| File | Description |
|------|-------------|
| `pr_issue_labeler.yml` | Auto-label PRs/issues by author type |
| `community_close_stale_issues.yml` | Close stale issues after warning period |
| `community_pr_board.yml` | Add community PRs to project board |
| `community_pr_board_refresh.yml` | Refresh community PR board |
| `community_update_all_top_ranking_issues.yml` | Update top ranking issues |
| `community_update_weekly_top_ranking_issues.yml` | Update weekly top ranking issues |
| `good_first_issue_notifier.yml` | Notify on good first issues |
| `comment_on_potential_duplicate_issues.yml` | Comment on potential duplicate issues |
| `track_duplicate_bot_effectiveness.yml` | Track duplicate detection effectiveness |
| `update_duplicate_magnets.yml` | Update duplicate issue magnets |
| `catch_blank_issues.yml` | Catch blank issue submissions |
| `congrats.yml` | Congratulations message for milestones |
| `add_commented_closed_issue_to_project.yml` | Add closed commented issues to project |
| `triage_project_sync.yml` | Sync triage project board |
| `stale-pr-reminder.yml` | Remind about stale PRs |
| `docs_suggestions.yml` | Docs suggestions workflow |

### Notifications

| File | Description |
|------|-------------|
| `slack_notify_first_responders.yml` | Slack notification for first responders |
| `slack_notify_label_created.yml` | Slack notification on label creation |
| `hotfix-review-monitor.yml` | Monitor hotfix PRs needing post-merge review |
