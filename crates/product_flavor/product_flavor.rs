//! Compile-time product identity and language-focus configuration.

use std::borrow::Cow;

include!("generated_product.rs");

/// The selected channel's stable data/config namespace.
pub const SELECTED_DATA_NAMESPACE: &str = if cfg!(product_release_channel = "dev") {
    DEV_DATA_NAMESPACE
} else if cfg!(product_release_channel = "nightly") {
    NIGHTLY_DATA_NAMESPACE
} else if cfg!(product_release_channel = "preview") {
    PREVIEW_DATA_NAMESPACE
} else {
    DATA_NAMESPACE
};

/// The selected channel's temporary updater directory prefix.
pub const SELECTED_AUTO_UPDATE_DIR_PREFIX: &str = if cfg!(product_release_channel = "dev") {
    DEV_AUTO_UPDATE_DIR_PREFIX
} else if cfg!(product_release_channel = "nightly") {
    NIGHTLY_AUTO_UPDATE_DIR_PREFIX
} else if cfg!(product_release_channel = "preview") {
    PREVIEW_AUTO_UPDATE_DIR_PREFIX
} else {
    AUTO_UPDATE_DIR_PREFIX
};

/// The selected channel's remote-server installation directory.
pub const SELECTED_REMOTE_SERVER_DIR: &str = if cfg!(product_release_channel = "dev") {
    DEV_REMOTE_SERVER_DIR
} else if cfg!(product_release_channel = "nightly") {
    NIGHTLY_REMOTE_SERVER_DIR
} else if cfg!(product_release_channel = "preview") {
    PREVIEW_REMOTE_SERVER_DIR
} else {
    REMOTE_SERVER_DIR
};

/// The selected channel's WSL remote-server installation directory.
pub const SELECTED_REMOTE_WSL_SERVER_DIR: &str = if cfg!(product_release_channel = "dev") {
    DEV_REMOTE_WSL_SERVER_DIR
} else if cfg!(product_release_channel = "nightly") {
    NIGHTLY_REMOTE_WSL_SERVER_DIR
} else if cfg!(product_release_channel = "preview") {
    PREVIEW_REMOTE_WSL_SERVER_DIR
} else {
    REMOTE_WSL_SERVER_DIR
};

/// A release channel used to derive operating-system identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    /// Development builds.
    Dev,
    /// Nightly builds.
    Nightly,
    /// Preview builds.
    Preview,
    /// Stable builds.
    Stable,
}

impl Channel {
    fn suffix(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Nightly => "nightly",
            Self::Preview => "preview",
            Self::Stable => "stable",
        }
    }
}

/// Returns the channel-specific display name.
pub fn display_name(channel: Channel) -> &'static str {
    match channel {
        Channel::Dev => DEV_DISPLAY_NAME,
        Channel::Nightly => NIGHTLY_DISPLAY_NAME,
        Channel::Preview => PREVIEW_DISPLAY_NAME,
        Channel::Stable => DISPLAY_NAME,
    }
}

/// Returns the channel-specific bundle/application identifier.
pub fn app_id(channel: Channel) -> &'static str {
    match channel {
        Channel::Dev => DEV_BUNDLE_IDENTIFIER,
        Channel::Nightly => NIGHTLY_BUNDLE_IDENTIFIER,
        Channel::Preview => PREVIEW_BUNDLE_IDENTIFIER,
        Channel::Stable => BUNDLE_IDENTIFIER,
    }
}

/// Returns the channel-specific data namespace.
pub fn data_namespace(channel: Channel) -> String {
    match channel {
        Channel::Stable => DATA_NAMESPACE.to_string(),
        _ => format!("{DATA_NAMESPACE}-{}", channel.suffix()),
    }
}

/// Returns the namespace used by IPC, mutexes, and single-instance endpoints.
pub fn instance_namespace(channel: Channel) -> String {
    format!("{}-{}", ID, channel.suffix())
}

/// Returns the channel-specific updater namespace.
pub fn update_namespace(channel: Channel) -> String {
    format!("{UPDATE_NAMESPACE}/{}", channel.suffix())
}

/// Returns the product-private CLI transport scheme.
pub fn cli_url_scheme() -> String {
    format!("{URL_SCHEME}-cli")
}

/// Builds a public product URL from an exact path/query suffix.
pub fn url(suffix: &str) -> String {
    format!("{URL_PREFIX}{suffix}")
}

/// Maps selected-product schemes to the internal legacy parser namespaces.
pub fn normalize_url(url: &str) -> Cow<'_, str> {
    if let Some(suffix) = url.strip_prefix(URL_PREFIX) {
        Cow::Owned(format!("zed://{suffix}"))
    } else if let Some(suffix) = url.strip_prefix(CLI_URL_PREFIX) {
        Cow::Owned(format!("zed-cli://{suffix}"))
    } else if let Some(suffix) = url.strip_prefix(DOCK_ACTION_URL_PREFIX) {
        Cow::Owned(format!("zed-dock-action://{suffix}"))
    } else {
        Cow::Borrowed(url)
    }
}

/// Returns the product-private dock-action transport scheme.
pub fn dock_action_url_scheme() -> String {
    format!("{URL_SCHEME}-dock-action")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn technical_identity_is_independent_of_display_name() {
        assert_eq!(ID, "rust");
        assert!(!BUNDLE_IDENTIFIER.contains(&DISPLAY_NAME.to_lowercase()));
        assert!(!DATA_NAMESPACE.contains(&DISPLAY_NAME.to_lowercase()));
        assert!(!URL_SCHEME.contains(&DISPLAY_NAME.to_lowercase()));
    }

    #[test]
    fn release_channels_have_distinct_identities() {
        let channels = [
            Channel::Dev,
            Channel::Nightly,
            Channel::Preview,
            Channel::Stable,
        ];
        let application_ids = channels.map(app_id);
        let data_namespaces = channels.map(data_namespace);
        assert_eq!(
            application_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            4
        );
        assert_eq!(
            data_namespaces
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            4
        );
    }

    #[test]
    fn project_configuration_namespace_remains_zed() {
        assert_eq!(".zed", ".zed");
    }

    #[test]
    fn selected_data_namespace_matches_release_channel() {
        let expected = if cfg!(product_release_channel = "dev") {
            "ide-rust-dev"
        } else if cfg!(product_release_channel = "nightly") {
            "ide-rust-nightly"
        } else if cfg!(product_release_channel = "preview") {
            "ide-rust-preview"
        } else {
            "ide-rust"
        };
        assert_eq!(SELECTED_DATA_NAMESPACE, expected);
    }
}
