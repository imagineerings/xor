---
title: CLI Reference
description: "Reference for Baymax's command-line interface (CLI), including opening files and directories, integrating with tools, and controlling Baymax from scripts."
---

# CLI Reference

Use Baymax's command-line interface (CLI) to open files and directories, integrate with other tools, and control Baymax from scripts.

## Installation

**macOS:** Run the {#action cli::InstallCliBinary} command from the command palette ({#kb command_palette::Toggle}) to install the `baymax` CLI to `/usr/local/bin/baymax`.

**Linux:** The CLI is included with Baymax packages. The binary name may vary by distribution (commonly `baymax` or `baymaxitor`).

**Windows:** The CLI is included with Baymax. Add Baymax's installation directory to your PATH, or use the full path to `baymax.exe`.

## Usage

```sh
baymax [OPTIONS] [PATHS]...
```

## Opening Files and Directories

Open a file:

```sh
baymax myfile.txt
```

Open a directory as a workspace:

```sh
baymax ~/projects/myproject
```

Open multiple files or directories:

```sh
baymax file1.txt file2.txt ~/projects/myproject
```

Open a file at a specific line and column:

```sh
baymax myfile.txt:42        # Open at line 42
baymax myfile.txt:42:10     # Open at line 42, column 10
```

## Options

### `-w`, `--wait`

Wait for all opened files to be closed before the CLI exits. When opening a directory, waits until the window is closed.

This is useful for integrating Baymax with tools that expect an editor to block until editing is complete (e.g., `git commit`):

```sh
export EDITOR="baymax --wait"
git commit  # Opens Baymax and waits for you to close the commit message file
```

### `-n`, `--new`

Open paths in a new workspace window, even if the paths are already open in an existing window:

```sh
baymax -n ~/projects/myproject
```

### `-a`, `--add`

Add paths to the currently focused workspace instead of opening a new window. When multiple workspace windows are open, files open in the focused window:

```sh
baymax -a newfile.txt
```

### `-r`, `--reuse`

Reuse an existing window, replacing its current workspace with the new paths:

```sh
baymax -r ~/projects/different-project
```

By default (without `-n`, `-a`, or `-r`), directories open in the current window's sidebar. You can change this default with the `cli_default_open_behavior` setting. See [Windows & Projects](../windows-and-projects.md) for more details.

### `--diff <OLD_PATH> <NEW_PATH>`

Open a diff view comparing two files. Can be specified multiple times:

```sh
baymax --diff file1.txt file2.txt
baymax --diff old.rs new.rs --diff old2.rs new2.rs
```

### `--foreground`

Run Baymax in the foreground, keeping the terminal attached. Useful for debugging:

```sh
baymax --foreground
```

### `--user-data-dir <DIR>`

Use a custom directory for all user data (database, extensions, logs) instead of the default location:

```sh
baymax --user-data-dir ~/.baymax-custom
```

Default locations:

- **macOS:** `~/Library/Application Support/Baymax`
- **Linux:** `$XDG_DATA_HOME/baymax` (typically `~/.local/share/baymax`)
- **Windows:** `%LOCALAPPDATA%\Baymax`

### `-v`, `--version`

Print Baymax's version and exit:

```sh
baymax --version
```

### `--uninstall`

Uninstall Baymax and remove all related files (macOS and Linux only):

```sh
baymax --uninstall
```

### `--baymax <PATH>`

Specify a custom path to the Baymax application or binary:

```sh
baymax --baymax /path/to/Baymax.app myfile.txt
```

## Reading from Standard Input

Read content from stdin by passing `-` as the path:

```sh
echo "Hello, World!" | baymax -
cat myfile.txt | baymax -
ps aux | baymax -
```

This creates a temporary file with the stdin content and opens it in Baymax.

## URL Handling

The CLI can open `baymax://`, `file://`, and `ssh://` URLs:

```sh
baymax baymax://settings
baymax file:///Users/whatever/.zshrc
baymax ssh://me@example.com/abs/path
baymax ssh://me@example.com:/abs/path
baymax ssh://me@example.com/~/project
baymax ssh://me@example.com:~/project
```

## Using Baymax as Your Default Editor

Set Baymax as your default editor for Git and other tools:

```sh
export EDITOR="baymax --wait"
export VISUAL="baymax --wait"
```

Add these lines to your shell configuration file (e.g., `~/.bashrc`, `~/.zshrc`).

## macOS: Switching Release Channels

On macOS, you can launch a specific release channel by passing the channel name as the first argument:

```sh
baymax --stable myfile.txt
baymax --preview myfile.txt
baymax --nightly myfile.txt
```

## WSL Integration (Windows)

On Windows, the CLI supports opening paths from WSL distributions. This is handled automatically when launching Baymax from within WSL.

## Exit Codes

| Code | Meaning                           |
| ---- | --------------------------------- |
| `0`  | Success                           |
| `1`  | Error (details printed to stderr) |

When using `--wait`, the exit code reflects whether the files were saved before closing.
