---
title: Debugging Crashes
description: "Guide to debugging crashes for Sim development."
---

# Debugging Crashes

When Sim panics or crashes, it sends a message to a sidecar process that inspects the editor's memory and creates a [minidump](https://chromium.googlesource.com/breakpad/breakpad/+/master/docs/getting_started_with_breakpad.md#the-minidump-file-format) in `~/Library/Logs/Sim` or `$XDG_DATA_HOME/sim/logs`. You can use this minidump to generate backtraces for all thread stacks.

If telemetry is enabled, Sim uploads these reports when you restart the app. Reports are sent to a [Slack channel](https://simtropolis.slack.com/archives/C0977J9MA1Y) and to [Sentry](https://sim-dev.sentry.io/issues) (both are Sim-staff-only).

These crash reports include useful data, but they are hard to read without spans or symbol information. You can still analyze them locally by downloading source and an unstripped binary (or separate symbols file) for your Sim release, then running:

```sh
zstd -d ~/.local/share/sim/<uuid>.dmp -o minidump.dmp
minidump-stackwalk minidump.dmp
```

Alongside the minidump in your logs directory, you should also see a `<uuid>.json` file with metadata such as the panic message, span, and system specs.

## Using a Debugger

If you can reproduce the crash consistently, use a debugger to inspect program state at the crash point.

For setup details, see [Using a debugger](./debuggers.md#debugging-panics-and-crashes).
