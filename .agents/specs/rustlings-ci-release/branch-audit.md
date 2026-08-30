# Remote branch reconciliation and cleanup record

Inspected on 2026-08-30 after fetching every origin branch over SSH. These are all five `origin/codex/*` branches, not a name-filtered subset. PR #11 is the reconciliation PR. No branch may be deleted until that PR is merged and its checks pass. Before deletion, re-read the remote tip and open PR state; delete only with an exact expected-SHA lease. A changed tip or new relevant open PR requires another audit.

## Recovery and disposition

| Remote branch | Full inspected tip SHA | Associated PR | Equivalent merged tree | Deletion rationale after PR #11 lands |
| --- | --- | --- | --- | --- |
| `codex/automate-copper-releases` | `7f7a426d19b9b46bec9c8644e8d42cb263549483` | [#6, merged](https://github.com/simtropolis/made/pull/6) | `0ef17a204f01120557e9dfde5d71463c607d46e8` | Automatic semantic releases/tags, recovery, serialization, and asset checks retained; the single validation worker is made lighter and strict aggregation is derived from its dependencies. |
| `codex/fix-windows-release-long-paths` | `dade4a17ead9abde9cde5697984509c1c3193329` | [#7, merged](https://github.com/simtropolis/made/pull/7) | `e14da55c4c879bb4bf26c23627d322546bb65d7b` | Windows Git long-path setup retained in the automatic release generator. |
| `codex/isolate-cargo-about-windows` | `4155a9d020d4c3e6ca579e8d5874d41639319da9` | [#8, merged](https://github.com/simtropolis/made/pull/8) | `b43dd8b79bd6c2669d4c7cc647654efcce534666` | Isolated temporary cargo-about target and regression test retained; product output-path correction complements this fix. |
| `codex/allow-blueoak-license` | `ba8de148b52c09675555dcd2aaeded113ca7e2ff` | [#9, merged](https://github.com/simtropolis/made/pull/9) | `629eba4ad12a6e3819e70b19f2b304dc7ef756a8` | BlueOak license acceptance retained unchanged. |
| `codex/use-native-windows-cmake` | `28164cc21eeb352e3ba561e3556e75a52b2faf04` | [#10, merged](https://github.com/simtropolis/made/pull/10) | `80da798afd675424b5177e29f104566226d621ea` | Native Visual Studio CMake and its regression test retained. Broken clang-18 pin superseded by the runner's available Clang; Windows product paths corrected for MSVC. |

`git diff --exit-code <branch-tip> <equivalent-merged-tree>` returned zero with no diff for every row. Each equivalent commit is already an ancestor of main; cleanup does not rely on identical commit IDs or patch messages.

## Commit inventory and dependencies

All branches share `e9c18ee76fdb859de401165c3a180527a89f09b2` (root README). That already-merged upstream documentation is left unchanged during reconciliation.

- Automatic release: `79189ce1c5b55a6afe1a89f2fa1283b863541b7f` plus `7f7a426d19b9b46bec9c8644e8d42cb263549483` is tree-equivalent to `0ef17a204f01120557e9dfde5d71463c607d46e8`. Changes cover the workflow README, CI/release YAML and generators, root README, xtask manifest/entry points/products/release-version task, and Cargo.lock. The user subsequently approved automatic release/tag policy; it is retained, not rejected.
- Long paths: `dade4a17ead9abde9cde5697984509c1c3193329` equals `e14da55c4c879bb4bf26c23627d322546bb65d7b`; depends on the automatic-release line and changes the release generator/YAML.
- Cargo-about: `c48fa167eb0b12648b08186ec167e74f8684b1ba` plus `4155a9d020d4c3e6ca579e8d5874d41639319da9` equals `b43dd8b79bd6c2669d4c7cc647654efcce534666`; depends on automatic release and long paths, changing license generation and bundle regression tests.
- BlueOak: `ba8de148b52c09675555dcd2aaeded113ca7e2ff` equals `629eba4ad12a6e3819e70b19f2b304dc7ef756a8`; follows cargo-about and changes only license acceptance beyond that base.
- Native compilers: `1e5140ddb83128f469c0e53a7e759304d06581b9`, `5153335875076bd67ee1bc370b3aee66f1fc2910`, `12029f6911dfd9c6437cb54cd6153fbb5d23112e`, and `28164cc21eeb352e3ba561e3556e75a52b2faf04` equal the final main squash above; follow BlueOak and change the Windows bundler, bundle tests, release generator/YAML. The first two are useful; the Clang pin required correction.

## Evidence and limitations

All five branch tips passed ordinary CI, but the corresponding release attempts did not establish successful native packaging. The [final release attempt](https://github.com/simtropolis/made/actions/runs/33320838111) failed on absent Linux `clang-18` and Windows extended-length compiler paths. Those failure boundaries are corrected; native release success must still be observed in automatic post-merge runs. No release is manually dispatched as part of this work.

Preserve `main`, `dev`, `rustlings`, all tags, and every local branch. Preserve any codex branch if fresh inspection reveals valuable unmerged work, a new unresolved decision, or active relevant PR work. The table records cleanup eligibility and recovery SHAs, not a claim that deletion has already happened.
