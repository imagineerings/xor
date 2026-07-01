---
name: land
description: Monitor Baymax PR checks, address CI failures with up to 3 fix-and-push cycles, and squash-merge the PR once checks pass.
---

# land skill

Use this skill when a PR is open and ready to merge into `main`. The skill
monitors GitHub checks for the current PR, fixes failures when appropriate, and
performs the squash merge.

## 0. Pre-flight

Verify `gh` is available:

```bash
command -v gh >/dev/null 2>&1 || { echo "ERROR: 'gh' CLI not found in PATH. Install gh and ensure it is on PATH before invoking the land skill."; exit 1; }
```

If `gh` is absent, abort immediately. Do not proceed.

Check that the working tree does not contain uncommitted changes that would be
lost or accidentally pushed:

```bash
git status -sb
```

If local changes exist, commit them intentionally, stash them only at the user's
request, or abort and report the remaining files.

## 1. Identify the PR

Confirm the current branch has an open PR:

```bash
gh pr view --json number,url,headRefName,headRefOid,state,statusCheckRollup
```

If no PR exists or the PR is not open, surface an error and halt:

```text
ERROR: No open PR found for the current branch. Run the push skill first.
```

Note the PR number and head SHA.

## 2. Wait for PR checks

Baymax uses root GitHub workflows such as `run_tests.yml`, `release.yml`, and
mobile workflows when relevant. Do not hard-code the obsolete `server-ci`
workflow. Monitor the PR's actual checks:

```bash
gh pr checks --watch --fail-fast --interval 60
```

Interpret the result:

| Result | Action |
|---|---|
| All required checks pass | Proceed to step 4 |
| A check fails | Proceed to step 3 |
| Checks are cancelled | Surface error and abort |
| Checks do not complete within roughly 45 minutes | Abort and surface the PR URL and current check state |

If `gh pr checks --watch` is unavailable in the installed `gh` version, poll:

```bash
gh pr checks --json name,state,conclusion,link
```

Use the current PR checks rather than guessing workflow names.

## 3. Fix CI failures (maximum 3 cycles)

If a check fails, enter the fix-and-push loop. Track the cycle count and abort
after 3 failed fix cycles.

### 3a. Inspect failing checks

List failing checks:

```bash
gh pr checks --json name,state,conclusion,link
```

For GitHub Actions checks, open the failing run logs:

```bash
gh run view <RUN_ID> --log-failed
```

If the failing check link is not a GitHub Actions run, open or summarize the
linked provider output instead.

### 3b. Fix the root cause

Apply the smallest change that addresses the failing check. Do not make
unrelated refactors. Follow the same repo-specific validation rules used by the
`commit` skill before committing the fix.

### 3c. Commit the fix

Use the `commit` skill with a message like:

```text
fix(<scope>): address CI failure
```

### 3d. Push the branch

Use a normal push when history is linear:

```bash
git push origin HEAD
```

Use `--force-with-lease` only if you intentionally amended or rebased:

```bash
git push origin HEAD --force-with-lease
```

### 3e. Wait for checks again

Return to step 2. The timeout resets for each new pushed commit.

### 3f. Cycle limit exceeded

If checks still fail after 3 fix-and-push cycles:

```text
ERROR: CI still failing after 3 fix-and-push cycles (PR #<number>).
Last failing check: <check name/link>
Aborting land operation. Manual intervention required.
```

Do not merge.

## 4. Squash-merge the PR

Once required PR checks pass, squash-merge:

```bash
gh pr merge <PR_NUMBER> --squash --auto --delete-branch
```

Flags:
- `--squash`: combine all PR commits into a single commit on `main`
- `--auto`: merge automatically once required checks and reviews allow it
- `--delete-branch`: delete the remote branch after merge

Verify the merge:

```bash
gh pr view <PR_NUMBER> --json state,mergedAt
```

Confirm `state` is `MERGED`. If the merge command fails because of conflicts,
missing approvals, or branch protection, surface the full error and halt.

## Error handling summary

| Condition | Action |
|---|---|
| `gh` not in PATH | Abort immediately |
| No open PR | Abort and tell the user to run the push skill first |
| Dirty worktree | Abort unless the user asks to commit/stash specific files |
| Checks timeout | Abort with PR URL and current check state |
| Check cancelled | Abort with check name/link |
| 3 fix cycles exhausted | Abort and leave PR open |
| Merge command fails | Surface `gh` error output and halt |

Never silently swallow errors. Never bypass branch protection or CI.
