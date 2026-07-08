use anyhow::Result;
use gpui::{Context, EventEmitter, Task, TaskExt};

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
    /// Errors are logged through the GPUI task executor.
    pub fn broadcast(&mut self, message: OutgoingMessage, cx: &mut Context<Self>) {
        for handler in &self.gateways {
            let task = handler.send_message(message.clone());
            task.detach_and_log_err(cx);
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
            task.detach_and_log_err(cx);
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use gpui::AppContext as _;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug, Default)]
    struct HandlerState {
        started: bool,
        stopped: bool,
        sent_messages: Vec<OutgoingMessage>,
    }

    struct TestHandler {
        name: String,
        state: Arc<Mutex<HandlerState>>,
        send_error: Option<String>,
    }

    impl TestHandler {
        fn new(name: impl Into<String>) -> (Self, Arc<Mutex<HandlerState>>) {
            let state = Arc::new(Mutex::new(HandlerState::default()));
            (
                Self {
                    name: name.into(),
                    state: state.clone(),
                    send_error: None,
                },
                state,
            )
        }

        fn with_send_error(mut self, message: impl Into<String>) -> Self {
            self.send_error = Some(message.into());
            self
        }
    }

    impl GatewayHandler for TestHandler {
        fn name(&self) -> &str {
            &self.name
        }

        fn start(&mut self, _cx: &mut Context<GatewayManager>) -> Task<Result<()>> {
            self.state
                .lock()
                .expect("test state should not be poisoned")
                .started = true;
            Task::ready(Ok(()))
        }

        fn stop(&mut self) -> Task<Result<()>> {
            self.state
                .lock()
                .expect("test state should not be poisoned")
                .stopped = true;
            Task::ready(Ok(()))
        }

        fn send_message(&self, message: OutgoingMessage) -> Task<Result<()>> {
            self.state
                .lock()
                .expect("test state should not be poisoned")
                .sent_messages
                .push(message);

            if let Some(error) = &self.send_error {
                Task::ready(Err(anyhow!(error.clone())))
            } else {
                Task::ready(Ok(()))
            }
        }
    }

    #[test]
    fn register_and_unregister_handlers_by_name() {
        let mut manager = GatewayManager::new();
        let (telegram, _) = TestHandler::new("telegram");
        let (slack, _) = TestHandler::new("slack");

        manager.register(Box::new(telegram));
        manager.register(Box::new(slack));
        assert_eq!(manager.gateways.len(), 2);

        manager.unregister("telegram");
        assert_eq!(manager.gateways.len(), 1);
        assert_eq!(manager.gateways[0].name(), "slack");
    }

    #[gpui::test]
    fn start_broadcast_and_stop_handlers(cx: &mut gpui::TestAppContext) {
        let (handler, state) = TestHandler::new("telegram");
        let manager = cx.new(|_| {
            let mut manager = GatewayManager::new();
            manager.register(Box::new(handler));
            manager
        });
        let outgoing = OutgoingMessage {
            platform: "telegram".into(),
            platform_id: "chat-1".into(),
            text: "hello".into(),
            attachments: Vec::new(),
            reply_to: None,
        };

        manager.update(cx, |manager, cx| {
            manager.start_all(cx);
            manager.broadcast(outgoing.clone(), cx);
            let stop_tasks = manager.stop_all();
            assert_eq!(stop_tasks.len(), 1);
            for task in stop_tasks {
                task.detach_and_log_err(cx);
            }
        });
        cx.run_until_parked();

        let state = state.lock().expect("test state should not be poisoned");
        assert!(state.started);
        assert!(state.stopped);
        assert_eq!(state.sent_messages.len(), 1);
        assert_eq!(state.sent_messages[0].text, "hello");
    }

    #[gpui::test]
    fn route_message_emits_gateway_event(cx: &mut gpui::TestAppContext) {
        let manager = cx.new(|_| GatewayManager::new());
        let events = Arc::new(Mutex::new(Vec::new()));
        let subscription = {
            let events = events.clone();
            cx.update(|cx| {
                cx.subscribe(&manager, move |_, event, _| {
                    events
                        .lock()
                        .expect("test events should not be poisoned")
                        .push(event.clone());
                })
            })
        };
        let incoming = IncomingMessage {
            platform: "telegram".into(),
            platform_id: "chat-1".into(),
            user_id: "user-1".into(),
            text: "hello".into(),
            attachments: Vec::new(),
            timestamp: chrono::Utc::now(),
        };

        manager.update(cx, |manager, cx| {
            manager.route_message(incoming, cx);
        });
        cx.run_until_parked();

        let events = events.lock().expect("test events should not be poisoned");
        assert_eq!(events.len(), 1);
        match &events[0] {
            GatewayEvent::MessageReceived(message) => {
                assert_eq!(message.platform, "telegram");
                assert_eq!(message.platform_id, "chat-1");
                assert_eq!(message.user_id, "user-1");
                assert_eq!(message.text, "hello");
            }
        }
        drop(subscription);
    }

    #[gpui::test]
    fn broadcast_continues_after_handler_error(cx: &mut gpui::TestAppContext) {
        let (failing_handler, failing_state) = TestHandler::new("failing");
        let failing_handler = failing_handler.with_send_error("send failed");
        let (working_handler, working_state) = TestHandler::new("working");
        let manager = cx.new(|_| {
            let mut manager = GatewayManager::new();
            manager.register(Box::new(failing_handler));
            manager.register(Box::new(working_handler));
            manager
        });
        let outgoing = OutgoingMessage {
            platform: "telegram".into(),
            platform_id: "chat-1".into(),
            text: "hello".into(),
            attachments: Vec::new(),
            reply_to: None,
        };

        manager.update(cx, |manager, cx| {
            manager.broadcast(outgoing, cx);
        });
        cx.run_until_parked();

        assert_eq!(
            failing_state
                .lock()
                .expect("test state should not be poisoned")
                .sent_messages
                .len(),
            1
        );
        assert_eq!(
            working_state
                .lock()
                .expect("test state should not be poisoned")
                .sent_messages
                .len(),
            1
        );
    }

    #[gpui::test]
    fn message_round_trip_with_mock_agent(cx: &mut gpui::TestAppContext) {
        let (handler, handler_state) = TestHandler::new("telegram");
        let manager = cx.new(|_| {
            let mut manager = GatewayManager::new();
            manager.register(Box::new(handler));
            manager
        });
        let subscription = {
            let manager = manager.clone();
            cx.update(|cx| {
                cx.subscribe(&manager.clone(), move |_, event, cx| {
                    let GatewayEvent::MessageReceived(message) = event;
                    let response = OutgoingMessage {
                        platform: message.platform.clone(),
                        platform_id: message.platform_id.clone(),
                        text: format!("agent heard: {}", message.text),
                        attachments: Vec::new(),
                        reply_to: None,
                    };
                    manager.update(cx, |manager, cx| {
                        manager.broadcast(response, cx);
                    });
                })
            })
        };
        let incoming = IncomingMessage {
            platform: "telegram".into(),
            platform_id: "chat-1".into(),
            user_id: "user-1".into(),
            text: "hello".into(),
            attachments: Vec::new(),
            timestamp: chrono::Utc::now(),
        };

        manager.update(cx, |manager, cx| {
            manager.route_message(incoming, cx);
        });
        cx.run_until_parked();

        let state = handler_state
            .lock()
            .expect("test state should not be poisoned");
        assert_eq!(state.sent_messages.len(), 1);
        assert_eq!(state.sent_messages[0].platform, "telegram");
        assert_eq!(state.sent_messages[0].platform_id, "chat-1");
        assert_eq!(state.sent_messages[0].text, "agent heard: hello");
        drop(subscription);
    }
}
