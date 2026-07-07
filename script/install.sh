#!/usr/bin/env sh
set -eu

# Downloads a tarball from https://sim.dev/releases and unpacks it
# into ~/.local/. If you'd prefer to do this manually, instructions are at
# https://sim.dev/docs/linux.

main() {
    platform="$(uname -s)"
    arch="$(uname -m)"
    channel="${SIM_CHANNEL:-stable}"
    SIM_VERSION="${SIM_VERSION:-latest}"
    # Use TMPDIR if available (for environments with non-standard temp directories)
    if [ -n "${TMPDIR:-}" ] && [ -d "${TMPDIR}" ]; then
        temp="$(mktemp -d "$TMPDIR/sim-XXXXXX")"
    else
        temp="$(mktemp -d "/tmp/sim-XXXXXX")"
    fi

    if [ "$platform" = "Darwin" ]; then
        platform="macos"
    elif [ "$platform" = "Linux" ]; then
        platform="linux"
    else
        echo "Unsupported platform $platform"
        exit 1
    fi

    case "$platform-$arch" in
        macos-arm64* | linux-arm64* | linux-armhf | linux-aarch64)
            arch="aarch64"
            ;;
        macos-x86* | linux-x86* | linux-i686*)
            arch="x86_64"
            ;;
        *)
            echo "Unsupported platform or architecture"
            exit 1
            ;;
    esac

    if command -v curl >/dev/null 2>&1; then
        curl () {
            command curl -fL "$@"
        }
    elif command -v wget >/dev/null 2>&1; then
        curl () {
            wget -O- "$@"
        }
    else
        echo "Could not find 'curl' or 'wget' in your path"
        exit 1
    fi

    "$platform" "$@"

    if [ "$(command -v sim)" = "$HOME/.local/bin/sim" ]; then
        echo "Sim has been installed. Run with 'sim'"
    else
        echo "To run Sim from your terminal, you must add ~/.local/bin to your PATH"
        echo "Run:"

        case "$SHELL" in
            *zsh)
                echo "   echo 'export PATH=\$HOME/.local/bin:\$PATH' >> ~/.zshrc"
                echo "   source ~/.zshrc"
                ;;
            *fish)
                echo "   fish_add_path -U $HOME/.local/bin"
                ;;
            *)
                echo "   echo 'export PATH=\$HOME/.local/bin:\$PATH' >> ~/.bashrc"
                echo "   source ~/.bashrc"
                ;;
        esac

        echo "To run Sim now, '~/.local/bin/sim'"
    fi
}

linux() {
    if [ -n "${SIM_BUNDLE_PATH:-}" ]; then
        cp "$SIM_BUNDLE_PATH" "$temp/sim-linux-$arch.tar.gz"
    else
        echo "Downloading Sim version: $SIM_VERSION"
        curl "https://cloud.sim.dev/releases/$channel/$SIM_VERSION/download?asset=sim&arch=$arch&os=linux&source=install.sh" > "$temp/sim-linux-$arch.tar.gz"
    fi

    suffix=""
    if [ "$channel" != "stable" ]; then
        suffix="-$channel"
    fi

    appid=""
    case "$channel" in
      stable)
        appid="dev.sim.Sim"
        ;;
      nightly)
        appid="dev.sim.Sim-Nightly"
        ;;
      preview)
        appid="dev.sim.Sim-Preview"
        ;;
      dev)
        appid="dev.sim.Sim-Dev"
        ;;
      *)
        echo "Unknown release channel: ${channel}. Using stable app ID."
        appid="dev.sim.Sim"
        ;;
    esac

    # Unpack
    rm -rf "$HOME/.local/sim$suffix.app"
    mkdir -p "$HOME/.local/sim$suffix.app"
    tar -xzf "$temp/sim-linux-$arch.tar.gz" -C "$HOME/.local/"

    # Setup ~/.local directories
    mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"

    # Link the binary
    if [ -f "$HOME/.local/sim$suffix.app/bin/sim" ]; then
        ln -sf "$HOME/.local/sim$suffix.app/bin/sim" "$HOME/.local/bin/sim"
    else
        # support for versions before 0.139.x.
        ln -sf "$HOME/.local/sim$suffix.app/bin/cli" "$HOME/.local/bin/sim"
    fi

    # Copy .desktop file
    desktop_file_path="$HOME/.local/share/applications/${appid}.desktop"
    src_dir="$HOME/.local/sim$suffix.app/share/applications"
    if [ -f "$src_dir/${appid}.desktop" ]; then
        cp "$src_dir/${appid}.desktop" "${desktop_file_path}"
    else
        # Fallback for older tarballs
        cp "$src_dir/sim$suffix.desktop" "${desktop_file_path}"
    fi
    sed -i "s|Icon=sim|Icon=$HOME/.local/sim$suffix.app/share/icons/hicolor/512x512/apps/sim.png|g" "${desktop_file_path}"
    sed -i "s|Exec=sim|Exec=$HOME/.local/sim$suffix.app/bin/sim|g" "${desktop_file_path}"
}

macos() {
    echo "Downloading Sim version: $SIM_VERSION"
    curl "https://cloud.sim.dev/releases/$channel/$SIM_VERSION/download?asset=sim&os=macos&arch=$arch&source=install.sh" > "$temp/Sim-$arch.dmg"
    hdiutil attach -quiet "$temp/Sim-$arch.dmg" -mountpoint "$temp/mount"
    app="$(cd "$temp/mount/"; echo *.app)"
    echo "Installing $app"
    if [ -d "/Applications/$app" ]; then
        echo "Removing existing $app"
        rm -rf "/Applications/$app"
    fi
    ditto "$temp/mount/$app" "/Applications/$app"
    hdiutil detach -quiet "$temp/mount"

    mkdir -p "$HOME/.local/bin"
    # Link the binary
    ln -sf "/Applications/$app/Contents/MacOS/cli" "$HOME/.local/bin/sim"
}

main "$@"
