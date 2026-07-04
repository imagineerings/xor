use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for the gateway system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Telegram bot token (can also be set via TELEGRAM_BOT_TOKEN env var).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_bot_token: Option<String>,

    /// Polling interval for Telegram getUpdates in seconds.
    #[serde(
        default = "default_polling_interval",
        skip_serializing_if = "is_default_polling_interval"
    )]
    pub telegram_polling_interval_seconds: u64,

    /// Path to pairing persistence JSON file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing_file: Option<PathBuf>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            telegram_bot_token: None,
            telegram_polling_interval_seconds: 1,
            pairing_file: None,
        }
    }
}

fn default_polling_interval() -> u64 {
    1
}

fn is_default_polling_interval(v: &u64) -> bool {
    *v == 1
}

impl GatewayConfig {
    /// Load configuration from environment variables and optional config file.
    ///
    /// Environment variables:
    /// - `TELEGRAM_BOT_TOKEN` — Telegram bot token
    /// - `GATEWAY_TELEGRAM_POLLING_INTERVAL` — polling interval in seconds
    /// - `GATEWAY_PAIRING_FILE` — path to pairing persistence file
    pub fn from_env() -> Self {
        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());

        let telegram_polling_interval_seconds = std::env::var("GATEWAY_TELEGRAM_POLLING_INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let pairing_file = std::env::var("GATEWAY_PAIRING_FILE")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

        Self {
            telegram_bot_token,
            telegram_polling_interval_seconds,
            pairing_file,
        }
    }

    /// Returns `true` if the gateway has any configured channels.
    pub fn is_enabled(&self) -> bool {
        self.telegram_bot_token.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GatewayConfig::default();
        assert!(!config.is_enabled());
        assert_eq!(config.telegram_polling_interval_seconds, 1);
    }

    #[test]
    fn test_from_env() {
        unsafe {
            std::env::set_var("TELEGRAM_BOT_TOKEN", "test_token");
            std::env::set_var("GATEWAY_TELEGRAM_POLLING_INTERVAL", "5");
            std::env::set_var("GATEWAY_PAIRING_FILE", "/tmp/pairings.json");
        }

        let config = GatewayConfig::from_env();
        assert!(config.is_enabled());
        assert_eq!(config.telegram_bot_token, Some("test_token".into()));
        assert_eq!(config.telegram_polling_interval_seconds, 5);
        assert_eq!(config.pairing_file, Some("/tmp/pairings.json".into()));

        unsafe {
            std::env::remove_var("TELEGRAM_BOT_TOKEN");
            std::env::remove_var("GATEWAY_TELEGRAM_POLLING_INTERVAL");
            std::env::remove_var("GATEWAY_PAIRING_FILE");
        }
    }
}
