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
use gpui::{Context, Task};

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
