---
title: Uninstall
description: "This guide covers how to uninstall Baymax on different operating systems."
---

# Uninstall

This guide covers how to uninstall Baymax on different operating systems.

## macOS

### Standard Installation

If you installed Baymax by downloading it from the website:

1. Quit Baymax if it's running
2. Open Finder and go to your Applications folder
3. Drag Baymax to the Trash (or right-click and select "Move to Trash")
4. Empty the Trash

### Homebrew Installation

If you installed Baymax using Homebrew, use the following command:

```sh
brew uninstall --cask baymax
```

Or for the preview version:

```sh
brew uninstall --cask baymax@preview
```

### Removing User Data (Optional)

To completely remove all Baymax configuration files and data:

1. Open Finder
2. Press `Cmd + Shift + G` to open "Go to Folder"
3. Delete the following directories if they exist:
   - `~/Library/Application Support/Baymax`
   - `~/Library/Saved Application State/dev.baymax.Baymax.savedState`
   - `~/Library/Logs/Baymax`
   - `~/Library/Caches/dev.baymax.Baymax`
   - `~/Library/Caches/Baymax`
   - `~/.config/baymax`
   - `~/.local/state/Baymax`

## Linux

### Standard Uninstall

If Baymax was installed using the default installation script, run:

```sh
baymax --uninstall
```

You'll be prompted whether to keep or delete your preferences. After making a choice, you should see a message that Baymax was successfully uninstalled.

If the `baymax` command is not found in your PATH, try:

```sh
$HOME/.local/bin/baymax --uninstall
```

or:

```sh
$HOME/.local/baymax.app/bin/baymax --uninstall
```

### Package Manager

If you installed Baymax using a package manager (such as Flatpak, Snap, or a distribution-specific package manager), consult that package manager's documentation for uninstallation instructions.

### Manual Removal

If the uninstall command fails or Baymax was installed to a custom location, you can manually remove:

- Installation directory: `~/.local/baymax.app` (or your custom installation path)
- Binary symlink: `~/.local/bin/baymax`
- Configuration and data: `~/.config/baymax`

## Windows

### Standard Installation

1. Quit Baymax if it's running
2. Open Settings (Windows key + I)
3. Go to "Apps" > "Installed apps" (or "Apps & features" on Windows 10)
4. Search for "Baymax"
5. Click the three dots menu next to Baymax and select "Uninstall"
6. Follow the prompts to complete the uninstallation

Alternatively, you can:

1. Open the Start menu
2. Right-click on Baymax
3. Select "Uninstall"

### Removing User Data (Optional)

To completely remove all Baymax configuration files and data:

1. Press `Windows key + R` to open Run
2. Type `%APPDATA%` and press Enter
3. Delete the `Baymax` folder if it exists
4. Press `Windows key + R` again, type `%LOCALAPPDATA%` and press Enter
5. Delete the `Baymax` folder if it exists

## Troubleshooting

If you encounter issues during uninstallation:

- **macOS/Windows**: Ensure Baymax is completely quit before attempting to uninstall. Check Activity Manager (macOS) or Task Manager (Windows) for any running Baymax processes.
- **Linux**: If the uninstall script fails, check the error message and consider manual removal of the directories listed above.
- **All platforms**: If you want to start fresh while keeping Baymax installed, you can delete the configuration directories instead of uninstalling the application entirely.

For additional help, see our [Linux-specific documentation](./linux.md) or visit the [Baymax community](https://baymax.dev/community-links).
