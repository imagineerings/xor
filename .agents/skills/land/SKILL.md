---
name: land
description: Monitor the server-ci GitHub Actions workflow, address CI failures with up to 3 fix-and-push cycles, and squash-merge the PR once CI passes.
---

# land skill

Use this skill when the PR is open and you are ready to merge it into `main`. The skill monitors CI, fixes failures, and performs the squash-merge.

## 0. Pre-flight: verify `gh` is available

```bash
command -v gh >/dev/null 2>&1 || { echo "ERROR: 'gh' CLI not found in PATH. Install gh and ensure it is on PATH before invoking the land skill."; exit 1; }
```

If `gh` is absent, **abort immediately** with the error above. Do not proceed to any subsequent step.

## 1. Identify the PR

Confirm the current branch has an open PR:

```bash
gh pr view --json number,url,headRefName,state
```

If no PR exists or the PR is not open, surface an error and halt:

```
ERROR: No open PR found for the current branch. Run the push skill first.
```

Note the PR number for use in subsequent steps.

## 2. Wait for the `server-ci` workflow (30-minute timeout)

Poll the `server-ci` workflow run associated with the PR's head commit. Check every 60 seconds for up to 30 minutes (30 iterations).

```bash
# Get the head SHA of the PR branch
HEAD_SHA=$(git rev-parse HEAD)

# Poll loop — run this logic up to 30 times (once per minute)
gh run list --workflow=server-ci.yml --commit "$HEAD_SHA" --json status,conclusion,databaseId --limit 1
```

Interpret the `status` and `conclusion` fields:

| `status`     | `conclusion` | Action                                      |
|--------------|--------------|---------------------------------------------|
| `completed`  | `success`    | CI passed — proceed to step 4               |
| `completed`  | `failure`    | CI failed — proceed to step 3               |
| `completed`  | `cancelled`  | Surface error and abort                     |
| `in_progress`| —            | Wait 60 seconds and poll again              |
| `queued`     | —            | Wait 60 seconds and poll again              |

If 30 minutes elapse without a `completed/success` result:

```
ERROR: server-ci workflow did not pass within 30 minutes (PR #<number>, commit <sha>).
Timed out waiting for CI. Inspect the workflow at: <run URL>
Aborting land operation.
```

**Abort** and surface this error. Do not squash-merge.

## 3. Fix CI failures (maximum 3 cycles)

If CI completes with `conclusion: failure`, enter the fix-and-push loop. Track the cycle count; **abort after 3 cycles** without a passing CI run.

### 3a. Pull the CI logs

```bash
RUN_ID=$(gh run list --workflow=server-ci.yml --commit "$HEAD_SHA" --json databaseId --limit 1 --jq '.[0].databaseId')
gh run view "$RUN_ID" --log-failed
```

Read the log output carefully. Identify the failing step and the root cause.

### 3b. Fix the failing issue

Apply the minimal code change that addresses the root cause identified in the logs. Do not make unrelated changes.

### 3c. Commit the fix

Use the `commit` skill to stage and commit the fix with a message of the form:

```
fix(<scope>): address CI failure — <short description of fix>
```

### 3d. Re-push the branch

```bash
git push origin HEAD --force-with-lease
```

### 3e. Wait for the new CI run

Return to step 2 to wait for the new `server-ci` run triggered by the re-push. The 30-minute timeout resets for each new run.

### 3f. Cycle limit exceeded

If CI has failed after **3 complete fix-and-push cycles** (i.e., you have pushed 3 fix commits and CI has failed each time):

```
ERROR: CI still failing after 3 fix-and-push cycles (PR #<number>).
Last failing run: <run URL>
Aborting land operation. Manual intervention required.
```

**Abort** and surface this error. Do not squash-merge. Leave the PR open for human inspection.

## 4. Squash-merge the PR

Once `server-ci` completes with `conclusion: success`, squash-merge the PR:

```bash
gh pr merge <PR_NUMBER> --squash --auto --delete-branch
```

Flags:
- `--squash`: combine all commits into a single commit on `main`
- `--auto`: merge automatically once all required checks pass (safe to use even if CI just passed)
- `--delete-branch`: clean up the remote branch after merge

Verify the merge succeeded:

```bash
gh pr view <PR_NUMBER> --json state,mergedAt
```

Confirm `state` is `MERGED`. If the merge command fails (e.g., merge conflicts, branch protection rules), surface the error output and halt without retrying.

## Error handling summary

| Condition | Error message | Action |
|---|---|---|
| `gh` not in PATH | `ERROR: 'gh' CLI not found in PATH.` | Abort immediately |
| No open PR | `ERROR: No open PR found for the current branch.` | Abort |
| CI timeout (30 min) | `ERROR: server-ci workflow did not pass within 30 minutes.` | Abort |
| CI cancelled | `ERROR: server-ci workflow was cancelled.` | Abort |
| 3 fix cycles exhausted | `ERROR: CI still failing after 3 fix-and-push cycles.` | Abort |
| Merge command fails | Surface `gh` error output verbatim | Abort |

Never silently swallow errors. Always surface the full error message before halting.
