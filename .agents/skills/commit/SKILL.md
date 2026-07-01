---
name: commit
description: Stage and commit changes in Baymax with repo-appropriate validation, a conventional multi-line message, and Co-authored-by attribution.
---

# commit skill

Use this skill whenever you need to create a git commit in the Baymax workspace.

## 1. Validation gate

Before staging, review the changed files and run validation that matches the
change. Do not use stale `server` or `webapp` commands; this repository is a
Rust workspace with mobile subprojects.

Always run:

```bash
git diff --check
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

If a relevant check is unavailable or too expensive for the current environment,
state that explicitly in the final response. If a check fails, fix the issue and
rerun the check before staging. Do not stage or commit known-broken work unless
the user explicitly asks for a checkpoint commit.

## 2. Stage changes

Review the diff before staging:

```bash
git diff --stat
git status -sb
```

Stage only files relevant to the current change. Prefer explicit paths over
`git add .` so unrelated user or agent work is not accidentally included:

```bash
git add <file1> <file2> ...
```

After staging, verify the staged set:

```bash
git diff --cached --name-status
git diff --cached --check
```

If unrelated files are staged, unstage them before committing.

## 3. Write the commit message

Compose a conventional commit message following this structure:

```text
<type>(<scope>): <short summary>

<body - explain what changed and why, wrap at 72 chars>

Co-authored-by: Agent <agent@simtropolis.ai>
```

- **type**: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, or `ci`
- **scope**: the affected area, for example `mobile`, `settings_ui`, `remote`, `gpui`, `docs`, or `ci`
- **short summary**: imperative mood, 72 characters or fewer, no trailing period
- **body**: recommended for non-trivial changes
- The `Co-authored-by` trailer must always be present on its own line after a blank line.

Write the message to a temporary file so newlines are preserved:

```bash
TMPFILE=$(mktemp /tmp/commit-msg.XXXXXX)
cat > "$TMPFILE" << 'COMMIT_MSG'
<type>(<scope>): <short summary>

<body>

Co-authored-by: Agent <agent@simtropolis.ai>
COMMIT_MSG
```

## 4. Commit

Use `-F` to read the message from the temp file:

```bash
git commit -F "$TMPFILE"
rm -f "$TMPFILE"
```

If the sandbox cannot write to `.git`, rerun the same `git commit` command with
the required repository-write approval. Do not bypass hooks with `--no-verify`.

Verify the commit:

```bash
git log -1 --format="%H %s"
git log -1 --format="%b"
```

Confirm the `Co-authored-by: Agent` trailer appears. If it is missing, amend the
commit with a corrected message.

## Error handling

- If a validation command fails, read the output, fix the underlying issue, and rerun the relevant validation.
- If `git commit` fails because hooks reject the change, fix the hook failure and retry from validation.
- If unrelated worktree changes exist, leave them unstaged unless the user explicitly asks to include them.
- Never use `git reset --hard`, `git checkout --`, or `git clean` to prepare a commit unless the user explicitly requests it.
