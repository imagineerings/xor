---
title: Installation
description: Install Sim on macOS, Linux, and Windows, then verify the command-line launcher.
---

# Installation

Install Sim using the package manager or download flow that matches your platform. After installation, launch Sim once from the desktop app and once from a terminal to confirm both entry points work.

## macOS

Install the stable build with Homebrew:

```sh
brew install --cask sim
```

Install the preview build if you want earlier access to upcoming releases:

```sh
brew install --cask sim@preview
```

You can also download stable and preview builds from the Sim website. For platform requirements and older macOS support details, see the existing [macOS guide](../macos.md).

## Linux

Use the install script for common distributions:

```sh
curl -f https://sim.dev/install.sh | sh
```

Install a preview channel build:

```sh
curl -f https://sim.dev/install.sh | SIM_CHANNEL=preview sh
```

Sim requires a Vulkan-capable graphics stack and desktop portals for system integration. See [Linux](../linux.md) for distribution-specific notes.

## Windows

Install with winget:

```powershell
winget install -e --id SimIndustries.Sim
```

Sim supports current Windows 11 releases and supported Windows 10 releases on x64 or Arm64 hardware. See [Windows](../windows.md) for platform details.

## Verify The Install

Open a terminal and run:

```sh
sim --version
```

Then open a project:

```sh
sim ~/projects/my-app
```

If the command is not found, restart your terminal and check whether the installer added Sim to your shell path.
