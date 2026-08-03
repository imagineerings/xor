# Baseline: Zed v1.11.3 upstream port

## Scope and authority

The sole Sim comparison target is the filesystem rooted at
`/Users/ahmad.vegah/repos/projects/sim-dev`. No remote Sim repository was
inspected. The upstream evidence source is the local, temporary Git object store
at `/tmp/zed-upstream-port`, populated only from
`https://github.com/zed-industries/zed.git` for the requested tags.

## Verified upstream endpoints

Both requested names resolve directly to commit objects (they are not annotated
tag objects in the fetched evidence):

| Tag | Verified commit SHA | Verification |
| --- | --- | --- |
| `v1.10.2` | `adc60ccf12e199b8828bad3abb2591e147034734` | `git rev-parse v1.10.2` and `git rev-parse 'v1.10.2^{}'` agree |
| `v1.11.3` | `952d712dac48a4af2c54fb22c82d82a9d69b72d4` | `git rev-parse v1.11.3` and `git rev-parse 'v1.11.3^{}'` agree |

The exclusive range `v1.10.2..v1.11.3` contains 160 commits. Its endpoint tree
diff contains 425 changed paths: 50 additions, 372 modifications, 2 deletions,
and 1 rename; the aggregate textual delta is 28,677 insertions and 9,160
deletions with no binary path reported by `git diff --numstat`.

## Local Sim identity and uncertainty

The supplied Sim tree has no `.git` directory or gitfile. Consequently all of
the following are unavailable and must remain explicitly unknown:

- local revision SHA;
- local branch;
- Git working-tree status and the tracked/untracked distinction;
- local commit history; and
- a history-established merge base with upstream Zed.

Commands including `git rev-parse HEAD`, `git branch --show-current`, and
`git status --porcelain=v1` fail with `fatal: not a git repository`. No SHA is
inferred from file timestamps, package versions, build artifacts, or upstream
similarity.

The available version evidence is narrower:

- `crates/sim/Cargo.toml` declares Sim `1.10.2`;
- `Cargo.lock` and `Cargo.lock.dev` identify the Sim package as `1.10.2`;
- `.agents/specs/comfy-parity/baseline.md` records a prior frozen filesystem
  fingerprint for a Sim 1.10.2 snapshot, while also recording that Git metadata
  was unavailable.

This establishes that the snapshot self-identifies as Sim 1.10.2. It does not
establish equality with the upstream Zed v1.10.2 tree or prove an exact upstream
base SHA.

## Filesystem preservation boundary

The previous frozen Sim fingerprint is
`99ceb40a1cc3359cde6e0865fe1b6138a06317d5fbd892f1595de10a96b07e9a` over
3,310 inputs. Re-running its documented content-fingerprint recipe against the
current filesystem produced
`c3c9ceeba9549b967d1da53826e688e0ad419ab969f40863dd6c23e6fdc0cb7d` over
4,086 inputs. This is a filesystem identity and preservation signal, not a Git
status substitute. Files were also observed changing during discovery.

Because there is no reliable tracked baseline, every pre-existing local file is
treated as authoritative user/Sim work. An upstream file may be replaced
unchanged only when its current Git blob hash exactly equals the verified
upstream v1.10.2 blob. All other overlaps require an explicit Sim-aware merge or
an exclusion decision.

## Endpoint tree relationship

For the 425 upstream-changed paths, comparison of current local content against
the two verified upstream trees produced:

| Relationship | All paths | Production/manifests/assets subset |
| --- | ---: | ---: |
| Current local blob equals upstream v1.10.2 | 106 | 94 |
| Current local blob equals upstream v1.11.3 | 0 | 0 |
| Current local blob differs from both endpoints | 259 | 162 |
| Target path is absent locally | 60 | 16 |
| **Total** | **425** | **272** |

The production subset is defined for this audit as `crates/**`, root Cargo
manifests/lockfile, and `assets/**`. Documentation, scripts, tooling, CI, and
generated/configuration paths remain in the full reconciliation and are not
silently excluded.

## Repository instructions applied

- Root `AGENTS.md` is a symlink to `.rules` and governs the repository.
- `docs/AGENTS.md` and `docs/.rules` additionally govern any approved `docs/**`
  edits.
- `WORKFLOW.md` governs local task planning. Its Linear-backed claim operations
  are not authorized because this delivery explicitly forbids external-system
  mutation; read-only planning/check commands and local `finish` gates remain
  available.
- Nested `projects/comfy/**` instructions apply only if those paths are touched;
  this upstream Zed range does not itself target them.

## Reproduction commands

```sh
git -C /tmp/zed-upstream-port rev-parse v1.10.2 'v1.10.2^{}'
git -C /tmp/zed-upstream-port rev-parse v1.11.3 'v1.11.3^{}'
git -C /tmp/zed-upstream-port rev-list --count v1.10.2..v1.11.3
git -C /tmp/zed-upstream-port diff --name-status --find-renames v1.10.2..v1.11.3
git -C /tmp/zed-upstream-port diff --numstat v1.10.2..v1.11.3
```

Local Git identity commands are intentionally recorded as failed evidence, not
as values to be filled by inference.
