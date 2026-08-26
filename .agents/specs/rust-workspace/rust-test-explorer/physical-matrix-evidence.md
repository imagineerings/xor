# Rust tools physical project-mode evidence

Status on 2026-08-26: implementation and local automation are available; physical remote certification is not complete.

Run the common coordinator from the repository root:

```sh
./script/test-rust-tools-environments --matrix --offline
```

The fixture is `crates/project/test_data/rust_test_provider/physical-workspace`. It has no external dependencies. The coordinator exercises offline structured discovery, unit/integration/ignored execution, cancellation, and stale-generation rejection. It reports these cells separately:

| Cell | Required environment | Remaining physical checklist |
| --- | --- | --- |
| Local | macOS, Linux, or Windows Zed build with `rust-tools` | Open the fixture in Zed; verify discovery, Run, Cancel, reconnect/reopen stale rejection, Debug, terminal reveal, and source navigation. Record OS/architecture, Zed revision, Rust version, and date. |
| SSH/headless | A separate supported macOS/Linux host, `ZED_RUST_TOOLS_SSH_HOST`, and `ZED_RUST_TOOLS_SSH_REPO` | Open the remote checkout using Zed SSH. Verify the task terminal, Cargo process, artifacts, and DAP process exist only on the remote host. Disconnect during discovery, reconnect, and confirm the late generation is rejected. |
| WSL | Windows with WSL, `ZED_RUST_TOOLS_WSL_DISTRO`, and `ZED_RUST_TOOLS_WSL_REPO` | Open the WSL project through its supported Zed project representation and repeat discovery/run/cancel/reconnect/debug checks. Confirm no Windows-local Cargo fallback. |
| Development container | A running supported container, `ZED_RUST_TOOLS_CONTAINER`, and `ZED_RUST_TOOLS_CONTAINER_REPO` | Open the container project through its supported Zed project representation and repeat discovery/run/cancel/reconnect/debug checks. Confirm no host-local Cargo fallback. |
| Multiplayer | Two clients and a shared writable project; set `ZED_RUST_TOOLS_MULTIPLAYER_EVIDENCE` to the completed record | From the guest, run and cancel one case, reject read-only execution, disconnect the authoritative host, and confirm no guest-local fallback or cross-peer cancellation. |

After every cell has dated physical results, rerun with `ZED_RUST_TOOLS_REQUIRE_PHYSICAL=1`. Do not check `rust-test-explorer/2.1`, `rust-test-explorer/2.2`, or `rust-tools-platform/2.3` until the required production-transport rows pass.
