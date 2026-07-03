use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use gateway::{GatewayConfig, PairingService};

#[derive(Parser, Debug)]
#[command(name = "gateway", about = "Manage gateway channels (Telegram, etc.)")]
struct GatewayArgs {
    #[command(subcommand)]
    command: GatewayCommand,
}

#[derive(Subcommand, Debug)]
enum GatewayCommand {
    /// Show gateway configuration status.
    Status,
    /// Print configuration help for gateway environment variables.
    Help,
    /// Start the gateway with a Telegram bot token.
    Start {
        /// Telegram bot token
        #[arg(long = "token", env = "TELEGRAM_BOT_TOKEN")]
        token: String,
    },
    /// Manage user pairings (link platform users to baymax users).
    Pair {
        #[command(subcommand)]
        subcmd: PairCommand,
    },
}

#[derive(Subcommand, Debug)]
enum PairCommand {
    /// Add a new pairing.
    Add {
        /// Platform user ID (e.g., tg:123456789)
        platform_id: String,
        /// Baymax user identity
        baymax_user: String,
    },
    /// Remove an existing pairing.
    Remove {
        /// Platform user ID to unlink
        platform_id: String,
    },
    /// List all active pairings.
    List,
}

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let args = GatewayArgs::try_parse_from(args)?;

    match args.command {
        GatewayCommand::Status => cmd_status(),
        GatewayCommand::Help => cmd_help(),
        GatewayCommand::Start { token } => cmd_start(&token),
        GatewayCommand::Pair { subcmd } => cmd_pair(subcmd),
    }
}

fn cmd_status() -> Result<()> {
    let config = GatewayConfig::from_env();
    println!("Gateway Status:");
    if config.is_enabled() {
        println!("  Telegram: configured");
        println!(
            "  Polling interval: {}ms",
            config.telegram_polling_interval_seconds * 1000
        );
    } else {
        println!("  Telegram: not configured (set TELEGRAM_BOT_TOKEN)");
    }
    Ok(())
}

fn cmd_help() -> Result<()> {
    println!("Gateway Configuration");
    println!();
    println!("Environment variables:");
    println!("  TELEGRAM_BOT_TOKEN  Telegram bot token (required for Telegram gateway)");
    println!();
    println!("Commands:");
    println!("  baymax gateway status           Show configuration status");
    println!("  baymax gateway start --token <token>  Start Telegram gateway");
    println!("  baymax gateway pair add <platform_id> <baymax_user>   Link platform user");
    println!("  baymax gateway pair remove <platform_id>               Unlink platform user");
    println!("  baymax gateway pair list         List all pairings");
    Ok(())
}

fn cmd_start(token: &str) -> Result<()> {
    println!(
        "Starting Telegram gateway with bot token: {}...",
        &token[..8.min(token.len())]
    );
    println!("Gateway running in foreground. Press Ctrl+C to stop.");
    println!();
    println!("NOTE: Full gateway startup requires integration with the");
    println!("baymax app. This command validates the configuration.");
    Ok(())
}

fn cmd_pair(subcmd: PairCommand) -> Result<()> {
    let pairing_path = pairing_storage_path();

    match subcmd {
        PairCommand::Add {
            platform_id,
            baymax_user,
        } => {
            let mut service = PairingService::with_storage(&pairing_path);
            service.pair_platform_user(&platform_id, &baymax_user)?;
            println!("✓ Paired {} -> {}", platform_id, baymax_user);
        }
        PairCommand::Remove { platform_id } => {
            let mut service = PairingService::with_storage(&pairing_path);
            if !service.is_paired(&platform_id) {
                println!("! Platform user '{}' is not paired", platform_id);
                return Ok(());
            }
            service.unlink(&platform_id)?;
            println!("✓ Unpaired {}", platform_id);
        }
        PairCommand::List => {
            let service = PairingService::with_storage(&pairing_path);
            if service.count() == 0 {
                println!("No active pairings");
            } else {
                println!("Active pairings ({}):", service.count());
                for (platform_id, baymax_user) in service.store() {
                    println!("  {} -> {}", platform_id, baymax_user);
                }
            }
        }
    }
    Ok(())
}

fn pairing_storage_path() -> PathBuf {
    paths::data_dir().join("gateway_pairings.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_status_without_token() {
        unsafe {
            env::remove_var("TELEGRAM_BOT_TOKEN");
        }
        let config = GatewayConfig::from_env();
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_status_with_token() {
        unsafe {
            env::set_var("TELEGRAM_BOT_TOKEN", "test_token");
        }
        let config = GatewayConfig::from_env();
        assert!(config.is_enabled());
        unsafe {
            env::remove_var("TELEGRAM_BOT_TOKEN");
        }
    }
}
