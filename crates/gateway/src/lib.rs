mod config;
mod manager;
mod pairing;
mod telegram;
mod telegram_format;
mod types;

pub use config::*;
pub use manager::*;
pub use pairing::*;
pub use telegram::*;
pub use telegram_format::*;
pub use types::*;

use anyhow::Result;
use gpui::{App, AppContext as _, Context, Entity, Task};
use std::time::Duration;

/// A handler for a specific messaging platform (e.g., Telegram).
///
/// Implementors manage the platform connection, receive incoming
/// messages and forward them to the manager, and send outgoing
/// messages from the manager to the platform.
pub trait GatewayHandler: Send {
    /// Human-readable name for this handler (e.g., "telegram").
    fn name(&self) -> &str;

    /// Start the handler, establishing a connection to the platform.
    ///
    /// The handler is given access to the manager context so it can
    /// route incoming messages back during its lifecycle via
    /// [`GatewayManager::route_message`].
    fn start(&mut self, cx: &mut Context<manager::GatewayManager>) -> Task<Result<()>>;

    /// Gracefully stop the handler, closing any connections.
    fn stop(&mut self) -> Task<Result<()>>;

    /// Send an outgoing message to the platform.
    fn send_message(&self, message: OutgoingMessage) -> Task<Result<()>>;
}

pub struct GlobalGatewayManager(Entity<GatewayManager>);

impl gpui::Global for GlobalGatewayManager {}

impl GatewayManager {
    pub fn try_global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalGatewayManager>()
            .map(|global| global.0.clone())
    }
}

pub fn init(config: GatewayConfig, cx: &mut App) -> Option<Entity<GatewayManager>> {
    if !config.is_enabled() {
        return None;
    }

    let manager = cx.new(|_| {
        let mut manager = GatewayManager::new();
        if let Some(token) = config.telegram_bot_token {
            let telegram = TelegramGateway::new(token).with_polling_interval(Duration::from_secs(
                config.telegram_polling_interval_seconds,
            ));
            manager.register(Box::new(telegram));
        }
        manager
    });

    manager.update(cx, |manager, cx| manager.start_all(cx));
    cx.set_global(GlobalGatewayManager(manager.clone()));
    Some(manager)
}
