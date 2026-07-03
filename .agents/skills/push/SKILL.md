---
name: push
description: Push the current Baymax branch to origin and create or update a GitHub PR using gh, with repo-appropriate validation and PR hygiene.
---

# push skill

Use this skill to push the current branch to `origin` and open or update a
GitHub pull request.

## 1. Pre-push validation gate

Before pushing, inspect the current branch and worktree:

```bash
git branch --show-current
git status -sb
```

If there are uncommitted changes, do not push them implicitly. Commit relevant
changes first, or stop and report the remaining files.

Run validation appropriate to the changes in the branch. Do not use stale
`server` or `webapp` commands; this repository is a Rust workspace with mobile
subprojects.

Always run:

```bash
git diff --check
git diff --cached --check
```

Then run the relevant checks:

| Changed area | Validation |
|---|---|
| Rust crates, `Cargo.toml`, `Cargo.lock` | `cargo fmt --all -- --check` and `./script/clippy` |
| Rust tests or behavior with meaningful risk | Relevant `cargo test ...` or `cargo nextest ...` command |
| Mobile scripts/workflows/specs | `mobile/scripts/tests/run.sh`, `mobile/scripts/mobile-readiness-check.sh`, and YAML parse for touched workflow/checklist files |
| Android app/build files | `mobile/scripts/android-test.sh` and, when feasible, `mobile/scripts/android-build.sh --variant debug --artifact apk --version 1.0.0 --build-number 1` |
| iOS project/build files on macOS | `mobile/scripts/ios-build.sh --configuration Debug --version 1.0.0 --build-number 1` |
| Docs/spec-only changes | `git diff --check`; add targeted parser/check commands when the docs are structured files |

If a relevant check cannot run in the current environment, say so in the PR body
under Testing. If a relevant check fails, abort the push, fix it, and retry from
the validation gate.

## 2. Verify `gh` authentication

Verify `gh` is available:

```bash
command -v gh >/dev/null 2>&1 || { echo "ERROR: gh CLI not found in PATH. Install gh before using the push skill."; exit 1; }
```

Verify authentication:

```bash
gh auth status
```

If `gh auth status` fails, either authenticate with `gh auth login` or set
`GH_TOKEN`/`GITHUB_TOKEN` in the environment. If only `GITHUB_TOKEN` is set,
export it for `gh`:

```bash
export GH_TOKEN="$GITHUB_TOKEN"
```

Abort on authentication errors.

## 3. Push the branch

Use a normal upstream push for new or linear branches:

```bash
git push --set-upstream origin HEAD
```

Use `--force-with-lease` only if you intentionally amended or rebased commits
that were already pushed:

```bash
git push --force-with-lease origin HEAD
```

Never use plain `--force`.

## 4. Compose the PR title and body

Follow the repo PR hygiene rules:

- Use a clear, correctly capitalized, imperative PR title.
- Do not use conventional commit prefixes in PR titles (`fix:`, `feat:`, `docs:`, etc.).
- Do not use trailing punctuation in PR titles.
- Optionally prefix with a crate or area when one scope is clear, for example `mobile: Add Android APK release artifact`.
- Include `Release Notes:` as the final section.
- Use exactly one release-notes bullet:
  - `- Added ...`, `- Fixed ...`, or `- Improved ...` for user-facing changes
  - `- N/A` for docs-only or other non-user-facing changes

If `.github/pull_request_template.md` exists, use it as the starting point and
fill in every item. Specifically:

- Check off each self-review checklist item that applies (`[x]`) after
  verifying it. Leave unchecked (`[ ]`) only items that genuinely do not
  apply, with a comment explaining why.
- Replace `#ISSUE` with the Linear issue identifier (e.g., `SIM-5`) or the
  GitHub issue number when this PR closes a tracked issue.
- Fill in the `Release Notes:` section with exactly one bullet.

If no template exists, write a concise temporary PR body with:
If `.github/pull_request_template.md` exists, use it as the starting point.
Each checklist item must be honestly evaluated against the diff:

- Check off items that are satisfied: `[x] I've reviewed my own diff...`
- Leave items unchecked and explain why if they don't apply (e.g., no unsafe
  blocks, no UI changes, no performance impact)
- Replace `Closes #ISSUE` with `Closes #<PR-number>` if there is one, or
  remove the line entirely

The PR body must contain the full completed checklist, not a shortened version.

Otherwise, write a concise temporary PR body with:

```markdown
## Summary

- ...

## Testing

- ...

Release Notes:

- ...
```

Use a temporary file outside the repo, for example:

```bash
PR_BODY_FILE=$(mktemp /tmp/pr-body.XXXXXX.md)
```

## 5. Create or update the PR

Check whether the current branch already has a PR:

```bash
gh pr view --json number,url,state,title
```

If no PR exists, create one:

```bash
gh pr create \
  --title "<PR title>" \
  --body-file "$PR_BODY_FILE" \
  --base main
```

If a PR exists, update it:

```bash
gh pr edit \
  --title "<PR title>" \
  --body-file "$PR_BODY_FILE"
```

After creating or updating, retrieve the URL:

```bash
gh pr view --json url -q .url
```

Report the PR URL to the user.

## 6. Error handling

| Condition | Action |
|---|---|
| Dirty worktree | Abort unless the user asks to commit specific files first |
| Relevant validation fails | Fix failures before pushing |
| `gh` not in PATH | Abort with the install/auth requirement |
| `gh` authentication fails | Abort and ask for auth/token |
| `git push` rejected | Investigate; pull/rebase only with user consent when needed |
| Force push needed | Use `--force-with-lease`, never `--force` |
| `gh pr create/edit` fails | Surface the full `gh` error and halt |

Never silently continue after a failed push or PR command.
