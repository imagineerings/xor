use std::path::PathBuf;

use anyhow::{Context as _, Result};
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
    /// Load configuration from a JSON config file.
    ///
    /// A missing file is treated as default configuration so startup can call
    /// this unconditionally.
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read gateway config {}", path.display()))?;
        serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse gateway config {}", path.display()))
    }

    /// Load configuration from a JSON file, then apply environment overrides.
    pub fn from_env_and_file(path: Option<PathBuf>) -> Result<Self> {
        let mut config = match path {
            Some(path) => Self::from_file(path)?,
            None => Self::default(),
        };

        config.apply_env_overrides();
        Ok(config)
    }

    /// Persist this configuration as pretty-printed JSON.
    pub fn save_to_file(&self, path: impl Into<PathBuf>) -> Result<()> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, contents)
            .with_context(|| format!("failed to write gateway config {}", path.display()))
    }

    /// Load configuration from environment variables and optional config file.
    ///
    /// Environment variables:
    /// - `TELEGRAM_BOT_TOKEN` — Telegram bot token
    /// - `GATEWAY_TELEGRAM_POLLING_INTERVAL` — polling interval in seconds
    /// - `GATEWAY_PAIRING_FILE` — path to pairing persistence file
    pub fn from_env() -> Self {
        let mut config = Self::default();
        config.apply_env_overrides();
        config
    }

    /// Returns `true` if the gateway has any configured channels.
    pub fn is_enabled(&self) -> bool {
        self.telegram_bot_token.is_some()
    }

    fn apply_env_overrides(&mut self) {
        if let Some(token) = std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
        {
            self.telegram_bot_token = Some(token);
        }

        if let Some(interval) = std::env::var("GATEWAY_TELEGRAM_POLLING_INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
        {
            self.telegram_polling_interval_seconds = interval;
        }

        if let Some(pairing_file) = std::env::var("GATEWAY_PAIRING_FILE")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
        {
            self.pairing_file = Some(pairing_file);
        }
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

    #[test]
    fn test_file_config_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gateway.json");
        let config = GatewayConfig {
            telegram_bot_token: Some("file_token".into()),
            telegram_polling_interval_seconds: 7,
            pairing_file: Some(dir.path().join("pairings.json")),
        };

        config.save_to_file(&path).unwrap();
        let loaded = GatewayConfig::from_file(&path).unwrap();

        assert_eq!(loaded.telegram_bot_token, Some("file_token".into()));
        assert_eq!(loaded.telegram_polling_interval_seconds, 7);
        assert_eq!(loaded.pairing_file, Some(dir.path().join("pairings.json")));
    }

    #[test]
    fn test_env_overrides_file_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gateway.json");
        GatewayConfig {
            telegram_bot_token: Some("file_token".into()),
            telegram_polling_interval_seconds: 7,
            pairing_file: Some(dir.path().join("file_pairings.json")),
        }
        .save_to_file(&path)
        .unwrap();

        unsafe {
            std::env::set_var("TELEGRAM_BOT_TOKEN", "env_token");
            std::env::set_var("GATEWAY_TELEGRAM_POLLING_INTERVAL", "9");
            std::env::set_var("GATEWAY_PAIRING_FILE", dir.path().join("env_pairings.json"));
        }

        let config = GatewayConfig::from_env_and_file(Some(path)).unwrap();

        assert_eq!(config.telegram_bot_token, Some("env_token".into()));
        assert_eq!(config.telegram_polling_interval_seconds, 9);
        assert_eq!(
            config.pairing_file,
            Some(dir.path().join("env_pairings.json"))
        );

        unsafe {
            std::env::remove_var("TELEGRAM_BOT_TOKEN");
            std::env::remove_var("GATEWAY_TELEGRAM_POLLING_INTERVAL");
            std::env::remove_var("GATEWAY_PAIRING_FILE");
        }
    }
}
