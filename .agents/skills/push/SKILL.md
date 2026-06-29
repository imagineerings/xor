---
name: push
description: Push the current branch to origin and create or update a GitHub PR using the gh CLI. Runs pre-push test validation and handles PR body templating and authentication.
---

# Push Skill

Use this skill to push your branch to `origin` and open or update a GitHub pull request.

## Pre-push Validation Gate

Before pushing, run both test suites. Both must pass.

```bash
cd server && make test-server
```

```bash
cd webapp && bun run test
```

If either command exits non-zero, **abort the push**. Fix all failures and re-run the failing suite before retrying. Do not push a branch with failing tests.

## Verify gh CLI is Available

Before any `gh` operation, confirm the binary is present:

```bash
command -v gh >/dev/null 2>&1 || { echo "ERROR: gh CLI not found in PATH. Install gh before using the push skill."; exit 1; }
```

If `gh` is absent, abort immediately with the error above. Do not attempt any GitHub operations.

## Verify GITHUB_TOKEN

All `gh` CLI operations require a valid `GITHUB_TOKEN`. Check before proceeding:

```bash
if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "ERROR: GITHUB_TOKEN is not set. Set GITHUB_TOKEN before using the push skill."
  exit 1
fi
```

Export it so `gh` picks it up automatically:

```bash
export GH_TOKEN="$GITHUB_TOKEN"
```

If `GITHUB_TOKEN` is absent or `gh` reports an authentication error, abort immediately. Do **not** write any partial output to files in the workspace.

## Push the Branch

```bash
git push --set-upstream origin HEAD
```

Use `--force-with-lease` instead of `--force` if you need to overwrite a previous push on the same branch:

```bash
git push --force-with-lease origin HEAD
```

## Compose the PR Body

Check whether the repository provides a PR template:

```bash
if [ -f .github/pull_request_template.md ]; then
  PR_BODY_FILE=".github/pull_request_template.md"
else
  PR_BODY_FILE=""
fi
```

- **If the template exists**: use it as the PR body. Fill in all placeholder sections (e.g. "Summary", "Testing", "Checklist") with information relevant to the changes in this branch.
- **If the template does not exist**: write a concise summary PR body covering: what changed, why it changed, and how it was tested. Write the body to a temporary file (e.g. `/tmp/pr_body_$$.md`) and pass it with `--body-file`.

## Create or Update the PR

**Create a new PR** (first push):

```bash
gh pr create \
  --title "<Linear issue title>" \
  --body-file "${PR_BODY_FILE:-/tmp/pr_body_$$.md}" \
  --base main
```

**Update an existing PR** (subsequent pushes — the branch already has an open PR):

```bash
gh pr edit \
  --body-file "${PR_BODY_FILE:-/tmp/pr_body_$$.md}"
```

Use the Linear issue identifier and title as the PR title (e.g. `SIM-123: Add user profile endpoint`).

## After the PR is Open

1. Note the PR URL printed by `gh pr create` (or retrieve it with `gh pr view --json url -q .url`).
2. Pass the PR URL to the `linear` skill to attach it to the issue and move the issue to "Human Review".

## Error Handling

| Condition | Action |
|---|---|
| `gh` not in PATH | Print `ERROR: gh CLI not found in PATH` and abort |
| `GITHUB_TOKEN` absent or invalid | Print `ERROR: GITHUB_TOKEN is not set` (or `authentication failed`) and abort; do not write partial output |
| Pre-push tests fail | Fix failures, then retry from the validation gate |
| `git push` rejected (non-lease conflict) | Investigate before force-pushing; prefer rebasing |
| `gh pr create` fails | Surface the full error from `gh`; do not silently continue |
