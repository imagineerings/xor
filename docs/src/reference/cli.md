---
title: CLI Reference
description: "Reference for Sim's command-line interface (CLI), including opening files and directories, integrating with tools, and controlling Sim from scripts."
---

# CLI Reference

Use Sim's command-line interface (CLI) to open files and directories, integrate with other tools, and control Sim from scripts.

## Installation

**macOS:** Run the {#action cli::InstallCliBinary} command from the command palette ({#kb command_palette::Toggle}) to install the `sim` CLI to `/usr/local/bin/sim`.

**Linux:** The CLI is included with Sim packages. The binary name may vary by distribution (commonly `sim` or `simitor`).

**Windows:** The CLI is included with Sim. Add Sim's installation directory to your PATH, or use the full path to `sim.exe`.

## Usage

```sh
sim [OPTIONS] [PATHS]...
```

## Opening Files and Directories

Open a file:

```sh
sim myfile.txt
```

Open a directory as a workspace:

```sh
sim ~/projects/myproject
```

Open multiple files or directories:

```sh
sim file1.txt file2.txt ~/projects/myproject
```

Open a file at a specific line and column:

```sh
sim myfile.txt:42        # Open at line 42
sim myfile.txt:42:10     # Open at line 42, column 10
```

## Options

### `-w`, `--wait`

Wait for all opened files to be closed before the CLI exits. When opening a directory, waits until the window is closed.

This is useful for integrating Sim with tools that expect an editor to block until editing is complete (e.g., `git commit`):

```sh
export EDITOR="sim --wait"
git commit  # Opens Sim and waits for you to close the commit message file
```

### `-n`, `--new`

Open paths in a new workspace window, even if the paths are already open in an existing window:

```sh
sim -n ~/projects/myproject
```

### `-a`, `--add`

Add paths to the currently focused workspace instead of opening a new window. When multiple workspace windows are open, files open in the focused window:

```sh
sim -a newfile.txt
```

### `-r`, `--reuse`

Reuse an existing window, replacing its current workspace with the new paths:

```sh
sim -r ~/projects/different-project
```

By default (without `-n`, `-a`, or `-r`), directories open in the current window's sidebar. You can change this default with the `cli_default_open_behavior` setting. See [Windows & Projects](../windows-and-projects.md) for more details.

### `--diff <OLD_PATH> <NEW_PATH>`

Open a diff view comparing two files. Can be specified multiple times:

```sh
sim --diff file1.txt file2.txt
sim --diff old.rs new.rs --diff old2.rs new2.rs
```

### `--foreground`

Run Sim in the foreground, keeping the terminal attached. Useful for debugging:

```sh
sim --foreground
```

### `--user-data-dir <DIR>`

Use a custom directory for all user data (database, extensions, logs) instead of the default location:

```sh
sim --user-data-dir ~/.sim-custom
```

Default locations:

- **macOS:** `~/Library/Application Support/Sim`
- **Linux:** `$XDG_DATA_HOME/sim` (typically `~/.local/share/sim`)
- **Windows:** `%LOCALAPPDATA%\Sim`

### `-v`, `--version`

Print Sim's version and exit:

```sh
sim --version
```

### `--uninstall`

Uninstall Sim and remove all related files (macOS and Linux only):

```sh
sim --uninstall
```

### `--sim <PATH>`

Specify a custom path to the Sim application or binary:

```sh
sim --sim /path/to/Sim.app myfile.txt
```

## Reading from Standard Input

Read content from stdin by passing `-` as the path:

```sh
echo "Hello, World!" | sim -
cat myfile.txt | sim -
ps aux | sim -
```

This creates a temporary file with the stdin content and opens it in Sim.

## URL Handling

The CLI can open `sim://`, `file://`, and `ssh://` URLs:

```sh
sim sim://settings
sim file:///Users/whatever/.zshrc
sim ssh://me@example.com/abs/path
sim ssh://me@example.com:/abs/path
sim ssh://me@example.com/~/project
sim ssh://me@example.com:~/project
```

## Using Sim as Your Default Editor

Set Sim as your default editor for Git and other tools:

```sh
export EDITOR="sim --wait"
export VISUAL="sim --wait"
```

Add these lines to your shell configuration file (e.g., `~/.bashrc`, `~/.zshrc`).

## macOS: Switching Release Channels

On macOS, you can launch a specific release channel by passing the channel name as the first argument:

```sh
sim --stable myfile.txt
sim --preview myfile.txt
sim --nightly myfile.txt
```

## WSL Integration (Windows)

On Windows, the CLI supports opening paths from WSL distributions. This is handled automatically when launching Sim from within WSL.

## Exit Codes

| Code | Meaning                           |
| ---- | --------------------------------- |
| `0`  | Success                           |
| `1`  | Error (details printed to stderr) |

When using `--wait`, the exit code reflects whether the files were saved before closing.
