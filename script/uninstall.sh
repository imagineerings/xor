#!/usr/bin/env sh
set -eu

# Uninstalls Baymax that was installed using the install.sh script

check_remaining_installations() {
    platform="$(uname -s)"
    if [ "$platform" = "Darwin" ]; then
        # Check for any Baymax variants in /Applications
        remaining=$(ls -d /Applications/Baymax*.app 2>/dev/null | wc -l)
        [ "$remaining" -eq 0 ]
    else
        # Check for any Baymax variants in ~/.local
        remaining=$(ls -d "$HOME/.local/baymax"*.app 2>/dev/null | wc -l)
        [ "$remaining" -eq 0 ]
    fi
}

prompt_remove_preferences() {
    printf "Do you want to keep your Baymax preferences? [Y/n] "
    read -r response
    case "$response" in
        [nN]|[nN][oO])
            rm -rf "$HOME/.config/baymax"
            echo "Preferences removed."
            ;;
        *)
            echo "Preferences kept."
            ;;
    esac
}

main() {
    platform="$(uname -s)"
    channel="${BAYMAX_CHANNEL:-stable}"

    if [ "$platform" = "Darwin" ]; then
        platform="macos"
    elif [ "$platform" = "Linux" ]; then
        platform="linux"
    else
        echo "Unsupported platform $platform"
        exit 1
    fi

    "$platform"

    echo "Baymax has been uninstalled"
}

linux() {
    suffix=""
    if [ "$channel" != "stable" ]; then
        suffix="-$channel"
    fi

    appid=""
    db_suffix="stable"
    case "$channel" in
      stable)
        appid="dev.baymax.Baymax"
        db_suffix="stable"
        ;;
      nightly)
        appid="dev.baymax.Baymax-Nightly"
        db_suffix="nightly"
        ;;
      preview)
        appid="dev.baymax.Baymax-Preview"
        db_suffix="preview"
        ;;
      dev)
        appid="dev.baymax.Baymax-Dev"
        db_suffix="dev"
        ;;
      *)
        echo "Unknown release channel: ${channel}. Using stable app ID."
        appid="dev.baymax.Baymax"
        db_suffix="stable"
        ;;
    esac

    # Remove the app directory
    rm -rf "$HOME/.local/baymax$suffix.app"

    # Remove the binary symlink
    rm -f "$HOME/.local/bin/baymax"

    # Remove the .desktop file
    rm -f "$HOME/.local/share/applications/${appid}.desktop"

    # Remove the database directory for this channel
    rm -rf "$HOME/.local/share/baymax/db/0-$db_suffix"

    # Remove socket file
    rm -f "$HOME/.local/share/baymax/baymax-$db_suffix.sock"

    # Remove the entire Baymax directory if no installations remain
    if check_remaining_installations; then
        rm -rf "$HOME/.local/share/baymax"
        prompt_remove_preferences
    fi

    rm -rf $HOME/.baymax_server
}

macos() {
    app="Baymax.app"
    db_suffix="stable"
    app_id="dev.baymax.Baymax"
    case "$channel" in
      nightly)
        app="Baymax Nightly.app"
        db_suffix="nightly"
        app_id="dev.baymax.Baymax-Nightly"
        ;;
      preview)
        app="Baymax Preview.app"
        db_suffix="preview"
        app_id="dev.baymax.Baymax-Preview"
        ;;
      dev)
        app="Baymax Dev.app"
        db_suffix="dev"
        app_id="dev.baymax.Baymax-Dev"
        ;;
    esac

    # Remove the app bundle
    if [ -d "/Applications/$app" ]; then
        rm -rf "/Applications/$app"
    fi

    # Remove the binary symlink
    rm -f "$HOME/.local/bin/baymax"

    # Remove the database directory for this channel
    rm -rf "$HOME/Library/Application Support/Baymax/db/0-$db_suffix"

    # Remove app-specific files and directories
    rm -rf "$HOME/Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.ApplicationRecentDocuments/$app_id.sfl"*
    rm -rf "$HOME/Library/Caches/$app_id"
    rm -rf "$HOME/Library/HTTPStorages/$app_id"
    rm -rf "$HOME/Library/Preferences/$app_id.plist"
    rm -rf "$HOME/Library/Saved Application State/$app_id.savedState"

    # Remove the entire Baymax directory if no installations remain
    if check_remaining_installations; then
        rm -rf "$HOME/Library/Application Support/Baymax"
        rm -rf "$HOME/Library/Logs/Baymax"

        prompt_remove_preferences
    fi

    rm -rf $HOME/.baymax_server
}

main "$@"
