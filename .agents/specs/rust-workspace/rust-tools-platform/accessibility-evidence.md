# Cargo and Tests panel accessibility evidence

Status on 2026-08-26: automated semantics are implemented; macOS VoiceOver and Windows NVDA certification is not complete.

Run the automated baseline first:

```sh
cargo test -p language_tools
cargo test -p cargo_ui
cargo test -p tasks_ui --features test-explorer test_explorer
```

For each supported stack, record the OS version, Zed revision, assistive-technology version, tester, date, and pass/fail result for every item below.

- Open Cargo and Tests without a pointer and confirm the panel landmark/name is announced.
- Move with Up, Down, Home, End, Left, and Right; confirm focus, selected item, hierarchy level, and expand/collapse state are announced once.
- Invoke Expand All, Collapse All, Refresh, and filters; confirm the new state and loading/current/empty/partial/stale/error/disconnected/mismatch status are announced.
- Select workspace, package, target, suite, and case rows; confirm labels are unambiguous when package names repeat.
- Traverse every Cargo action. Confirm unavailable actions announce the exact disabled reason, Clean announces its confirmation requirement, and coverage failure provides setup guidance.
- Run, cancel, rerun failed tests, reveal the terminal, navigate to source, and invoke supported/unsupported Debug; confirm state transitions and the doctest-specific disabled reason.
- Test a 10,000-row fixture with keyboard navigation and ensure focus does not enter offscreen or stale rows.
- Disconnect and reconnect the authoritative host; confirm stale or mismatch data is not announced as current.

Required physical environments:

- macOS with VoiceOver enabled and keyboard navigation active.
- Windows with NVDA enabled in the supported Zed Windows build.

Leave `rust-tools-platform/2.4` unchecked until both stacks have dated results and every unresolved failure is linked from this file.
