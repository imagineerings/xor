#!/usr/bin/env sh
set -eu

# Downloads a tarball from https://baymax.dev/releases and unpacks it
# into ~/.local/. If you'd prefer to do this manually, instructions are at
# https://baymax.dev/docs/linux.

main() {
    platform="$(uname -s)"
    arch="$(uname -m)"
    channel="${BAYMAX_CHANNEL:-stable}"
    BAYMAX_VERSION="${BAYMAX_VERSION:-latest}"
    # Use TMPDIR if available (for environments with non-standard temp directories)
    if [ -n "${TMPDIR:-}" ] && [ -d "${TMPDIR}" ]; then
        temp="$(mktemp -d "$TMPDIR/baymax-XXXXXX")"
    else
        temp="$(mktemp -d "/tmp/baymax-XXXXXX")"
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

    if [ "$(command -v baymax)" = "$HOME/.local/bin/baymax" ]; then
        echo "Baymax has been installed. Run with 'baymax'"
    else
        echo "To run Baymax from your terminal, you must add ~/.local/bin to your PATH"
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

        echo "To run Baymax now, '~/.local/bin/baymax'"
    fi
}

linux() {
    if [ -n "${BAYMAX_BUNDLE_PATH:-}" ]; then
        cp "$BAYMAX_BUNDLE_PATH" "$temp/baymax-linux-$arch.tar.gz"
    else
        echo "Downloading Baymax version: $BAYMAX_VERSION"
        curl "https://cloud.baymax.dev/releases/$channel/$BAYMAX_VERSION/download?asset=baymax&arch=$arch&os=linux&source=install.sh" > "$temp/baymax-linux-$arch.tar.gz"
    fi

    suffix=""
    if [ "$channel" != "stable" ]; then
        suffix="-$channel"
    fi

    appid=""
    case "$channel" in
      stable)
        appid="dev.baymax.Baymax"
        ;;
      nightly)
        appid="dev.baymax.Baymax-Nightly"
        ;;
      preview)
        appid="dev.baymax.Baymax-Preview"
        ;;
      dev)
        appid="dev.baymax.Baymax-Dev"
        ;;
      *)
        echo "Unknown release channel: ${channel}. Using stable app ID."
        appid="dev.baymax.Baymax"
        ;;
    esac

    # Unpack
    rm -rf "$HOME/.local/baymax$suffix.app"
    mkdir -p "$HOME/.local/baymax$suffix.app"
    tar -xzf "$temp/baymax-linux-$arch.tar.gz" -C "$HOME/.local/"

    # Setup ~/.local directories
    mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"

    # Link the binary
    if [ -f "$HOME/.local/baymax$suffix.app/bin/baymax" ]; then
        ln -sf "$HOME/.local/baymax$suffix.app/bin/baymax" "$HOME/.local/bin/baymax"
    else
        # support for versions before 0.139.x.
        ln -sf "$HOME/.local/baymax$suffix.app/bin/cli" "$HOME/.local/bin/baymax"
    fi

    # Copy .desktop file
    desktop_file_path="$HOME/.local/share/applications/${appid}.desktop"
    src_dir="$HOME/.local/baymax$suffix.app/share/applications"
    if [ -f "$src_dir/${appid}.desktop" ]; then
        cp "$src_dir/${appid}.desktop" "${desktop_file_path}"
    else
        # Fallback for older tarballs
        cp "$src_dir/baymax$suffix.desktop" "${desktop_file_path}"
    fi
    sed -i "s|Icon=baymax|Icon=$HOME/.local/baymax$suffix.app/share/icons/hicolor/512x512/apps/baymax.png|g" "${desktop_file_path}"
    sed -i "s|Exec=baymax|Exec=$HOME/.local/baymax$suffix.app/bin/baymax|g" "${desktop_file_path}"
}

macos() {
    echo "Downloading Baymax version: $BAYMAX_VERSION"
    curl "https://cloud.baymax.dev/releases/$channel/$BAYMAX_VERSION/download?asset=baymax&os=macos&arch=$arch&source=install.sh" > "$temp/Baymax-$arch.dmg"
    hdiutil attach -quiet "$temp/Baymax-$arch.dmg" -mountpoint "$temp/mount"
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
    ln -sf "/Applications/$app/Contents/MacOS/cli" "$HOME/.local/bin/baymax"
}

main "$@"
