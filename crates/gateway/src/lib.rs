mod types;

pub use types::*;

use anyhow::Result;
use gpui::{Context, Task};

/// Placeholder for the GatewayManager entity (defined in Task 2).
///
/// The GatewayHandler trait references this type so that handler
/// implementations can interact with the manager during startup.
/// When Task 2 implements `crates/gateway/src/manager.rs`, this
/// type will be replaced with the real `GatewayManager`.
pub struct GatewayManager;

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
    /// The handler is given access to the manager so it can route
    /// incoming messages back during its lifecycle.
    fn start(&mut self, cx: &mut Context<GatewayManager>) -> Task<Result<()>>;

    /// Gracefully stop the handler, closing any connections.
    fn stop(&mut self) -> Task<Result<()>>;

    /// Send an outgoing message to the platform.
    fn send_message(&self, message: OutgoingMessage) -> Task<Result<()>>;
}
