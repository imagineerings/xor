---
name: commit
description: Stage and commit changes with a conventional multi-line message, lint validation, and Co-authored-by attribution.
---

# commit skill

Use this skill whenever you need to create a git commit in the Simtropolis workspace.

## 1. Validation gate (run before staging anything)

Run both lint checks. If either exits non-zero, **do not stage or commit**. Fix all reported errors first, then re-run the checks until both pass.

```bash
cd server && make lint
cd webapp && bun run check
```

- If `make lint` fails: read the golangci-lint output, fix each reported issue in the relevant Go file(s), and re-run.
- If `bun run check` fails: read the ESLint/Stylelint output, fix each reported issue in the relevant TypeScript/SCSS file(s), and re-run.
- Only proceed to step 2 once both commands exit 0.

## 2. Stage changes

Review the diff before staging:

```bash
git diff --stat
```

Stage the files relevant to the current change. Prefer explicit paths over `git add .` to avoid accidentally staging unrelated files:

```bash
git add <file1> <file2> ...
```

## 3. Write the commit message

Compose a conventional commit message following this structure:

```
<type>(<scope>): <short summary>

<body — explain what changed and why, wrap at 72 chars>

Co-authored-by: Agent <agent@simtropolis.ai>
```

- **type**: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, or `ci`
- **scope**: the affected area, e.g. `server`, `webapp`, `api`, `e2e`
- **short summary**: imperative mood, ≤ 72 characters, no trailing period
- **body**: optional but recommended for non-trivial changes
- The `Co-authored-by` trailer **must always be present** on every commit message, on its own line after a blank line.

Write the message to a temporary file so that newlines are preserved exactly:

```bash
TMPFILE=$(mktemp /tmp/commit-msg.XXXXXX)
cat > "$TMPFILE" << 'COMMIT_MSG'
<type>(<scope>): <short summary>

<body>

Co-authored-by: Agent <agent@simtropolis.ai>
COMMIT_MSG
```

## 4. Commit

Use `-F` to read the message from the temp file (never `-m`, which collapses newlines):

```bash
git commit -F "$TMPFILE"
rm -f "$TMPFILE"
```

Verify the commit was recorded correctly:

```bash
git log -1 --format="%H %s"
git log -1 --format="%b"
```

Confirm the `Co-authored-by: Agent` trailer appears in the log output. If it is missing, amend the commit before proceeding:

```bash
git commit --amend -F <new-tmpfile>
```

## Error handling

- If `git commit` fails (e.g., pre-commit hook rejection), read the error output, fix the underlying issue, and retry from step 1.
- Never force-push or use `--no-verify` to bypass hooks.
- If the validation gate keeps failing after two fix attempts, surface the lint output to the user and halt.
