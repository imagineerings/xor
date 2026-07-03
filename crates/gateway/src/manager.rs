use anyhow::Result;
use gpui::{Context, EventEmitter, Task};

use crate::{GatewayHandler, IncomingMessage, OutgoingMessage};

/// Events emitted by the [`GatewayManager`].
#[derive(Clone, Debug)]
pub enum GatewayEvent {
    /// An incoming message received from a gateway handler that should be
    /// routed to the agent for processing.
    MessageReceived(IncomingMessage),
}

/// Central manager for gateway handlers.
///
/// The manager owns a set of [`GatewayHandler`] instances and provides
/// message routing between external messaging platforms and the agent.
///
/// It is designed to be used as an `Entity<GatewayManager>` following GPUI
/// patterns, so that the agent integration layer (Task 6) can subscribe to
/// [`GatewayEvent`]s emitted by this entity.
pub struct GatewayManager {
    gateways: Vec<Box<dyn GatewayHandler>>,
}

impl EventEmitter<GatewayEvent> for GatewayManager {}

impl GatewayManager {
    /// Create a new empty gateway manager.
    pub fn new() -> Self {
        Self {
            gateways: Vec::new(),
        }
    }

    /// Register a gateway handler.
    ///
    /// After registration, [`start_all`](Self::start_all) or the individual
    /// handler's [`GatewayHandler::start`] should be called to begin
    /// processing messages from the platform.
    pub fn register(&mut self, handler: Box<dyn GatewayHandler>) {
        self.gateways.push(handler);
    }

    /// Unregister a gateway handler by name.
    ///
    /// The handler's [`GatewayHandler::stop`] should be called before
    /// removal to allow a graceful shutdown of the platform connection.
    pub fn unregister(&mut self, name: &str) {
        self.gateways.retain(|h| h.name() != name);
    }

    /// Route an incoming message to the agent.
    ///
    /// Emits a [`GatewayEvent::MessageReceived`] event that the agent
    /// integration layer (Task 6) subscribes to. Returns immediately;
    /// the actual processing happens in the subscriber.
    pub fn route_message(&mut self, message: IncomingMessage, cx: &mut Context<Self>) {
        cx.emit(GatewayEvent::MessageReceived(message));
    }

    /// Broadcast an outgoing message to all registered gateway handlers.
    ///
    /// Each handler's [`GatewayHandler::send_message`] is called with a
    /// clone of the message. Tasks are detached and run in the background.
    /// Errors are silently logged by the handler.
    pub fn broadcast(&mut self, message: OutgoingMessage) {
        for handler in &self.gateways {
            let task = handler.send_message(message.clone());
            task.detach();
        }
    }

    /// Start all registered gateway handlers.
    ///
    /// Should be called after all handlers are registered. Each handler's
    /// [`GatewayHandler::start`] task is spawned via the GPUI context and
    /// runs on the foreground thread.
    pub fn start_all(&mut self, cx: &mut Context<Self>) {
        for handler in &mut self.gateways {
            let task = handler.start(cx);
            task.detach();
        }
    }

    /// Stop all registered gateway handlers.
    ///
    /// Returns the shutdown tasks from each handler. Callers should await
    /// the returned tasks to ensure a graceful shutdown.
    pub fn stop_all(&mut self) -> Vec<Task<Result<()>>> {
        self.gateways.iter_mut().map(|h| h.stop()).collect()
    }
}
