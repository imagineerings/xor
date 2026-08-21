---
name: pull
description: Fetch origin/main and merge it into the current branch, configuring zdiff3 conflict style and rerere before merging.
---

# pull skill

Use this skill whenever you need to sync the current branch with the latest `origin/main`.

## Steps

### 1. Configure conflict helpers

Before merging, enable `zdiff3` conflict style and `rerere` so that repeated conflict patterns are remembered and auto-resolved:

```bash
git config merge.conflictstyle zdiff3
git config rerere.enabled true
```

### 2. Fetch origin/main

```bash
git fetch origin main
```

### 3. Merge origin/main into the current branch

```bash
git merge origin/main
```

### 4. Resolve conflicts (if any)

If the merge produces conflicts:

1. Open each conflicted file. The `zdiff3` style shows the common ancestor in the conflict marker, making it easier to understand what both sides changed.
2. Resolve each conflict manually, keeping the correct combination of changes.
3. After resolving, stage the file:
   ```bash
   git add <file>
   ```
4. Once all conflicts are resolved, complete the merge:
   ```bash
   git merge --continue
   ```

`rerere` will record the resolution. If the same conflict appears again in a future merge, Git will apply the recorded resolution automatically.

### 5. Verify

After the merge completes, confirm the branch is up to date:

```bash
git log --oneline -5
```
