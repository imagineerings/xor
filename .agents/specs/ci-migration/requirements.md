# Requirements: CI/CD Migration Plan — Archived Workflows

## Introduction

The Baymax repository previously contained 40 GitHub Actions workflows in `.github/workflows/`. Most of these were auxiliary workflows for community management, release automation, extension CI, issue triage, deployment, and project maintenance. As part of reducing the CI surface to a bare minimum (build, test, publish to GitHub Releases), 38 of those workflows have been archived to `.github/workflows/archive/`. This document specifies the requirements for re-implementing those workflows when needed.

## Glossary

- **CI/CD pipeline**: The set of GitHub Actions workflows that build, test, and publish the Baymax editor.
- **Archived workflow**: A workflow file moved to `.github/workflows/archive/` but not currently active (GitHub Actions does not scan subdirectories).
- **Core workflow**: One of the two retained workflows (`run_tests.yml`, `release.yml`) that constitute the bare-minimum CI/CD.
- **Auto-generated workflow**: A workflow with the header `# Generated from xtask::workflows::...`, meaning it is produced by `cargo xtask workflows` and would be regenerated if that command is run.
- **Spec migration plan**: A document (requirements + design + tasks) describing how to reintroduce an archived workflow.

## Requirements

### R1: Restore On-Demand Bundling (`run_bundling.yml`)

**User Story:** As a developer, I want to trigger a full multi-platform bundle build by applying a label to a PR, so that I can verify release artifacts before merging.

#### Acceptance Criteria

1. WHEN a PR is labeled `run-bundling` THEN THE system SHALL build and upload bundles for Linux (aarch64 + x86_64), macOS (aarch64 + x86_64), and Windows (aarch64 + x86_64).
2. WHEN a PR labeled `run-bundling` is synchronized THEN THE system SHALL rebuild bundles.
3. WHEN the bundles complete THEN THE system SHALL upload them as workflow artifacts.
4. IF the `run-bundling` label is removed THEN THE system SHALL NOT cancel running bundle jobs.

### R2: Restore Nightly Releases (`release_nightly.yml`)

**User Story:** As a release manager, I want nightly builds to be automatically produced on a schedule, so that users can test the latest changes without waiting for a formal release.

#### Acceptance Criteria

1. WHEN the scheduled cron triggers (every 4 hours) THEN THE system SHALL check if a new nightly tag is needed.
2. IF a new nightly is needed THEN THE system SHALL run tests, build bundles for all platforms, and upload them to DigitalOcean Spaces.
3. WHEN a nightly build succeeds THEN THE system SHALL update the nightly Git tag.
4. IF the nightly build fails THEN THE system SHALL notify via Slack.

### R3: Restore Post-Release Automation (`after_release.yml`)

**User Story:** As a release manager, I want post-release tasks (Discord notification, Winget publishing, docs deployment, Sentry release) to run automatically when a release is published.

#### Acceptance Criteria

1. WHEN a GitHub Release is published THEN THE system SHALL refresh the cloud releases page.
2. WHEN a GitHub Release is published THEN THE system SHALL deploy docs to the appropriate channel (preview/stable).
3. WHEN a GitHub Release is published THEN THE system SHALL post a release announcement to Discord.
4. WHEN a GitHub Release is published THEN THE system SHALL publish the Windows package to Winget.
5. WHEN a GitHub Release is published THEN THE system SHALL create a Sentry release.
6. IF any post-release step fails THEN THE system SHALL notify via Slack.

### R4: Restore Collab Server Deployment (`deploy_collab.yml`)

**User Story:** As an infrastructure engineer, I want to deploy the collaboration server by pushing a tag, so that the collab service is updated in production.

#### Acceptance Criteria

1. WHEN a `collab-production` tag is pushed THEN THE system SHALL run style checks and tests.
2. WHEN style and tests pass THEN THE system SHALL build and publish a Docker image to DigitalOcean Container Registry.
3. WHEN the Docker image is published THEN THE system SHALL deploy it to the production Kubernetes cluster.

### R5: Restore Docs Deployment (`deploy_docs.yml`, `deploy_nightly_docs.yml`)

**User Story:** As a documentation maintainer, I want docs to be built and deployed to Cloudflare Pages on pushes to main, so that users always have up-to-date documentation.

#### Acceptance Criteria

1. WHEN a push to `main` occurs THEN THE system SHALL build the mdBook documentation.
2. WHEN the docs build succeeds THEN THE system SHALL deploy to Cloudflare Pages under the nightly channel.
3. WHEN a release is published THEN THE system SHALL deploy docs to the appropriate channel (preview or stable).

### R6: Restore Auto-Fix PRs (`autofix_pr.yml`)

**User Story:** As a developer, I want to trigger automated code fixes (clippy, formatting, cargo-machete) on a PR via workflow dispatch, so that I can quickly address CI issues.

#### Acceptance Criteria

1. WHEN the workflow is dispatched with a PR number THEN THE system SHALL run cargo fix, clippy fix, cargo-machete fix, prettier, and cargo fmt.
2. IF there are changes THEN THE system SHALL commit and push them back to the PR branch.

### R7: Restore Nix Build (`nix_build.yml`)

**User Story:** As a Nix user, I want to trigger a Nix build by applying a label to a PR, so that I can verify the Nix derivation before merging.

#### Acceptance Criteria

1. WHEN a PR is labeled `run-nix` or `run-bundling` THEN THE system SHALL build the Nix derivation for Linux and macOS.
2. WHEN the build completes THEN results SHALL be pushed to Cachix.

### R8: Restore Compliance Checks (`compliance_check.yml`)

**User Story:** As a compliance officer, I want weekly compliance checks to run automatically, so that release readiness is verified before tagging.

#### Acceptance Criteria

1. WHEN the scheduled cron triggers (weekly, Tuesday 17:30 UTC) THEN THE system SHALL run a compliance check against the latest preview tag.
2. WHEN the check completes THEN THE system SHALL upload a compliance report artifact.
3. IF the check fails THEN THE system SHALL notify via Slack.

### R9: Restore Extension CI (`extension_tests.yml`, `extension_bump.yml`, `extension_auto_bump.yml`, `publish_extension_cli.yml`, `extension_workflow_rollout.yml`)

**User Story:** As an extension developer, I want extension contributions to be tested, version-bumped, and published automatically, so that the extension ecosystem remains healthy.

#### Acceptance Criteria

1. WHEN an extension directory is changed in a PR THEN THE system SHALL run Rust checks and extension validation.
2. WHEN an extension version changes THEN THE system SHALL verify the version was bumped by the bot.
3. WHEN the extension CLI is tagged THEN THE system SHALL build, upload, and update references across repos.
4. WHEN an extension is ready to publish THEN THE system SHALL bump its version and create a release PR.

### R10: Restore Danger Checks (`danger.yml`)

**User Story:** As a project maintainer, I want Danger to run on every PR to main, so that common PR quality issues are caught automatically.

#### Acceptance Criteria

1. WHEN a PR is opened or synchronized against `main` THEN THE system SHALL run Danger checks.
2. WHEN a merge queue entry is created THEN THE system SHALL run Danger checks.

### R11: Restore PR/Issue Labeling (`pr_issue_labeler.yml`)

**User Story:** As a community manager, I want PRs and issues to be automatically labeled by author type (staff, bot, community champion, first contribution, guild), so that triage is streamlined.

#### Acceptance Criteria

1. WHEN a PR is opened THEN THE system SHALL apply the appropriate author label.
2. WHEN an issue is opened THEN THE system SHALL apply community champion label if applicable.

### R12: Restore Cherry-Pick Workflow (`cherry_pick.yml`)

**User Story:** As a release manager, I want to cherry-pick commits into release branches via workflow dispatch, so that hotfixes can be backported.

#### Acceptance Criteria

1. WHEN the workflow is dispatched with a commit SHA, branch, and channel THEN THE system SHALL cherry-pick the commit and push to the target branch.

### R13: Restore Version Bumping (`bump_baymax_version.yml`, `bump_patch_version.yml`, `bump_collab_staging.yml`)

**User Story:** As a release manager, I want version bumps to be automated, so that releases follow a consistent process.

#### Acceptance Criteria

1. WHEN the bump workflow is dispatched THEN THE system SHALL compute the next version and create branches/tags for main, preview, and/or stable.

### R14: Restore Community Management Workflows

**User Story:** As a community manager, I want automated triage, stale-issue closing, PR board management, and duplicate detection, so that the repository stays organized.

#### Acceptance Criteria

1. WHEN a stale issue is detected THEN THE system SHALL close it after a warning period.
2. WHEN a duplicate issue is detected THEN THE system SHALL comment with a reference.
3. WHEN a community PR is opened THEN THE system SHALL add it to the community PR board.
4. WHEN a good-first-issue label is added THEN THE system SHALL notify first-time contributors.

### R15: Restore Slack Notifications

**User Story:** As a team member, I want Slack notifications for important events (first responder paging, label creation, hotfix review reminders, workflow failures), so that the team can respond promptly.

#### Acceptance Criteria

1. WHEN a hotfix is merged without post-merge review THEN THE system SHALL notify the #pr-review-ops Slack channel.
2. WHEN a workflow fails THEN THE system SHALL notify the appropriate Slack channel.
3. WHEN a high-priority label is created THEN THE system SHALL notify first responders.

### R16: Restore Release Notification (`push_release_update_notification` - part of `release.yml`)

**Note:** This is part of the retained `release.yml` and does not need to be re-implemented. It is included here for completeness.

### R17: Ensure Auto-Generated Workflows are Not Overwritten

**User Story:** As a developer running `cargo xtask workflows`, I want the archived auto-generated workflows to not be silently regenerated into the active directory, so that the archive remains authoritative.

#### Acceptance Criteria

1. WHEN `cargo xtask workflows` is run THEN THE system SHALL only write to `.github/workflows/archive/` for archived workflows, OR the regeneration mechanism SHALL be updated to skip archived workflows.
2. IF the xtask regeneration cannot be modified THEN THE system SHALL document this in a README so developers know to re-archive after regeneration.
