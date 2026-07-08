use std::rc::Rc;

use acp_thread::{AgentConnection, LoadError};
use agent_servers::AcpConnection;
use agent_servers::{AgentServer, AgentServerDelegate};
use anyhow::Result;
use collections::HashMap;
use futures::{FutureExt, future::Shared};
use gpui::{App, AppContext, Context, Entity, EventEmitter, SharedString, Subscription, Task};

use project::{AgentServerStore, AgentServersUpdated, Project};
use watch::Receiver;

use crate::Agent;

pub enum AgentConnectionEntry {
    Connecting {
        connect_task: Shared<Task<Result<AgentConnectedState, LoadError>>>,
    },
    Connected(AgentConnectedState),
    Error {
        error: LoadError,
    },
}

#[derive(Clone)]
pub struct AgentConnectedState {
    pub connection: Rc<dyn AgentConnection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcpConnectionDetails {
    pub status: AgentConnectionStatus,
    pub agent_id: Option<SharedString>,
    pub agent_version: Option<SharedString>,
    pub auth_method_count: usize,
    pub supports_load_session: bool,
    pub supports_resume_session: bool,
    pub supports_close_session: bool,
    pub supports_session_history: bool,
}

impl AcpConnectionDetails {
    pub fn summary_label(&self) -> SharedString {
        match self.status {
            AgentConnectionStatus::Disconnected => "ACP disconnected".into(),
            AgentConnectionStatus::Connecting => "ACP connecting".into(),
            AgentConnectionStatus::Connected => {
                let agent = self
                    .agent_id
                    .as_ref()
                    .map(|agent_id| agent_id.to_string())
                    .unwrap_or_else(|| "agent".to_string());
                match &self.agent_version {
                    Some(version) => format!("ACP connected: {agent} ({version})").into(),
                    None => format!("ACP connected: {agent}").into(),
                }
            }
        }
    }

    pub fn tooltip_text(&self) -> SharedString {
        let status = match self.status {
            AgentConnectionStatus::Disconnected => "Disconnected",
            AgentConnectionStatus::Connecting => "Connecting",
            AgentConnectionStatus::Connected => "Connected",
        };
        let agent = self
            .agent_id
            .as_ref()
            .map(|agent_id| agent_id.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let version = self
            .agent_version
            .as_ref()
            .map(|version| version.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        format!(
            "ACP status: {status}\nAgent: {agent}\nVersion: {version}\nAuth methods: {}\nSession history: {}\nLoad/resume/close: {}/{}/{}",
            self.auth_method_count,
            bool_label(self.supports_session_history),
            bool_label(self.supports_load_session),
            bool_label(self.supports_resume_session),
            bool_label(self.supports_close_session),
        )
        .into()
    }
}

impl AgentConnectionEntry {
    pub fn wait_for_connection(&self) -> Shared<Task<Result<AgentConnectedState, LoadError>>> {
        match self {
            AgentConnectionEntry::Connecting { connect_task } => connect_task.clone(),
            AgentConnectionEntry::Connected(state) => Task::ready(Ok(state.clone())).shared(),
            AgentConnectionEntry::Error { error } => Task::ready(Err(error.clone())).shared(),
        }
    }

    pub fn status(&self) -> AgentConnectionStatus {
        match self {
            AgentConnectionEntry::Connecting { .. } => AgentConnectionStatus::Connecting,
            AgentConnectionEntry::Connected(_) => AgentConnectionStatus::Connected,
            AgentConnectionEntry::Error { .. } => AgentConnectionStatus::Disconnected,
        }
    }

    pub fn acp_details(&self) -> AcpConnectionDetails {
        match self {
            AgentConnectionEntry::Connecting { .. } => AcpConnectionDetails {
                status: AgentConnectionStatus::Connecting,
                agent_id: None,
                agent_version: None,
                auth_method_count: 0,
                supports_load_session: false,
                supports_resume_session: false,
                supports_close_session: false,
                supports_session_history: false,
            },
            AgentConnectionEntry::Connected(state) => {
                let connection = &state.connection;
                AcpConnectionDetails {
                    status: AgentConnectionStatus::Connected,
                    agent_id: Some(connection.agent_id().0.clone()),
                    agent_version: connection.agent_version(),
                    auth_method_count: connection.auth_methods().len(),
                    supports_load_session: connection.supports_load_session(),
                    supports_resume_session: connection.supports_resume_session(),
                    supports_close_session: connection.supports_close_session(),
                    supports_session_history: connection.supports_session_history(),
                }
            }
            AgentConnectionEntry::Error { .. } => AcpConnectionDetails {
                status: AgentConnectionStatus::Disconnected,
                agent_id: None,
                agent_version: None,
                auth_method_count: 0,
                supports_load_session: false,
                supports_resume_session: false,
                supports_close_session: false,
                supports_session_history: false,
            },
        }
    }
}

pub enum AgentConnectionEntryEvent {
    NewVersionAvailable(SharedString),
    LoadingStatusChanged(Option<SharedString>),
}

impl EventEmitter<AgentConnectionEntryEvent> for AgentConnectionEntry {}

#[derive(Clone)]
pub struct ActiveAcpConnection {
    pub agent_id: project::AgentId,
    pub connection: Rc<AcpConnection>,
}

pub struct AgentConnectionStore {
    project: Entity<Project>,
    entries: HashMap<Agent, Entity<AgentConnectionEntry>>,
    _subscriptions: Vec<Subscription>,
}

impl AgentConnectionStore {
    pub fn new(project: Entity<Project>, cx: &mut Context<Self>) -> Self {
        let agent_server_store = project.read(cx).agent_server_store().clone();
        let subscription = cx.subscribe(&agent_server_store, Self::handle_agent_servers_updated);
        Self {
            project,
            entries: HashMap::default(),
            _subscriptions: vec![subscription],
        }
    }

    pub fn project(&self) -> &Entity<Project> {
        &self.project
    }

    pub fn entry(&self, key: &Agent) -> Option<&Entity<AgentConnectionEntry>> {
        self.entries.get(key)
    }

    pub fn connection_status(&self, key: &Agent, cx: &App) -> AgentConnectionStatus {
        self.entries
            .get(key)
            .map(|entry| entry.read(cx).status())
            .unwrap_or(AgentConnectionStatus::Disconnected)
    }

    pub fn acp_connection_details(&self, key: &Agent, cx: &App) -> AcpConnectionDetails {
        self.entries
            .get(key)
            .map(|entry| entry.read(cx).acp_details())
            .unwrap_or_else(|| AcpConnectionDetails {
                status: AgentConnectionStatus::Disconnected,
                agent_id: None,
                agent_version: None,
                auth_method_count: 0,
                supports_load_session: false,
                supports_resume_session: false,
                supports_close_session: false,
                supports_session_history: false,
            })
    }

    pub fn agent_version(&self, key: &Agent, cx: &App) -> Option<SharedString> {
        match self.entries.get(key)?.read(cx) {
            AgentConnectionEntry::Connected(state) => state.connection.agent_version(),
            AgentConnectionEntry::Connecting { .. } | AgentConnectionEntry::Error { .. } => None,
        }
    }

    pub fn active_acp_connections(&self, cx: &App) -> Vec<ActiveAcpConnection> {
        self.entries
            .values()
            .filter_map(|entry| match entry.read(cx) {
                AgentConnectionEntry::Connected(state) => state
                    .connection
                    .clone()
                    .downcast::<AcpConnection>()
                    .map(|connection| ActiveAcpConnection {
                        agent_id: state.connection.agent_id(),
                        connection,
                    }),
                AgentConnectionEntry::Connecting { .. } | AgentConnectionEntry::Error { .. } => {
                    None
                }
            })
            .collect()
    }

    pub fn restart_connection(
        &mut self,
        key: Agent,
        server: Rc<dyn AgentServer>,
        cx: &mut Context<Self>,
    ) -> Entity<AgentConnectionEntry> {
        if let Some(entry) = self.entries.get(&key) {
            if matches!(entry.read(cx), AgentConnectionEntry::Connecting { .. }) {
                return entry.clone();
            }
        }

        self.entries.remove(&key);
        self.request_connection(key, server, cx)
    }

    pub fn request_connection(
        &mut self,
        key: Agent,
        server: Rc<dyn AgentServer>,
        cx: &mut Context<Self>,
    ) -> Entity<AgentConnectionEntry> {
        if let Some(entry) = self.entries.get(&key) {
            return entry.clone();
        }

        let (mut new_version_rx, mut loading_status_rx, connect_task) =
            self.start_connection(server, cx);
        let connect_task = connect_task.shared();

        let entry = cx.new(|_cx| AgentConnectionEntry::Connecting {
            connect_task: connect_task.clone(),
        });

        self.entries.insert(key.clone(), entry.clone());
        cx.notify();

        cx.spawn({
            let key = key.clone();
            let entry = entry.downgrade();
            async move |this, cx| match connect_task.await {
                Ok(connected_state) => {
                    this.update(cx, move |this, cx| {
                        if this.entries.get(&key) != entry.upgrade().as_ref() {
                            return;
                        }

                        entry
                            .update(cx, move |entry, cx| {
                                if let AgentConnectionEntry::Connecting { .. } = entry {
                                    *entry = AgentConnectionEntry::Connected(connected_state);
                                    cx.notify();
                                }
                            })
                            .ok();
                        cx.notify();
                    })
                    .ok();
                }
                Err(error) => {
                    this.update(cx, move |this, cx| {
                        if this.entries.get(&key) != entry.upgrade().as_ref() {
                            return;
                        }

                        entry
                            .update(cx, move |entry, cx| {
                                if let AgentConnectionEntry::Connecting { .. } = entry {
                                    *entry = AgentConnectionEntry::Error { error };
                                    cx.notify();
                                }
                            })
                            .ok();
                        this.entries.remove(&key);
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();

        cx.spawn({
            let key = key.clone();
            let entry = entry.downgrade();
            async move |this, cx| {
                while let Ok(version) = new_version_rx.recv().await {
                    let Some(version) = version else {
                        continue;
                    };

                    this.update(cx, move |this, cx| {
                        if this.entries.get(&key) != entry.upgrade().as_ref() {
                            return;
                        }

                        entry
                            .update(cx, move |_entry, cx| {
                                cx.emit(AgentConnectionEntryEvent::NewVersionAvailable(
                                    version.into(),
                                ));
                            })
                            .ok();
                        this.entries.remove(&key);
                        cx.notify();
                    })
                    .ok();
                    break;
                }
            }
        })
        .detach();

        cx.spawn({
            let entry = entry.downgrade();
            async move |this, cx| {
                while let Ok(status) = loading_status_rx.recv().await {
                    let status = status.map(SharedString::from);
                    let key = key.clone();
                    let entry = entry.clone();
                    this.update(cx, move |this, cx| {
                        if this.entries.get(&key) != entry.upgrade().as_ref() {
                            return;
                        }

                        entry
                            .update(cx, move |_entry, cx| {
                                cx.emit(AgentConnectionEntryEvent::LoadingStatusChanged(status));
                            })
                            .ok();
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();

        entry
    }

    fn handle_agent_servers_updated(
        &mut self,
        store: Entity<AgentServerStore>,
        _: &AgentServersUpdated,
        cx: &mut Context<Self>,
    ) {
        let store = store.read(cx);
        self.entries.retain(|key, _| match key {
            Agent::NativeAgent => true,
            Agent::Custom { id } => store.external_agents.contains_key(id),
            #[cfg(any(test, feature = "test-support"))]
            Agent::Stub => true,
        });
        cx.notify();
    }

    fn start_connection(
        &self,
        server: Rc<dyn AgentServer>,
        cx: &mut Context<Self>,
    ) -> (
        Receiver<Option<String>>,
        Receiver<Option<String>>,
        Task<Result<AgentConnectedState, LoadError>>,
    ) {
        let (new_version_tx, new_version_rx) = watch::channel::<Option<String>>(None);
        let (loading_status_tx, loading_status_rx) = watch::channel::<Option<String>>(None);

        let agent_server_store = self.project.read(cx).agent_server_store().clone();
        let delegate = AgentServerDelegate::new(
            agent_server_store,
            Some(new_version_tx),
            Some(loading_status_tx),
        );

        let connect_task = server.connect(delegate, self.project.clone(), cx);
        let connect_task = cx.spawn(async move |_this, _cx| match connect_task.await {
            Ok(connection) => Ok(AgentConnectedState { connection }),
            Err(err) => match err.downcast::<LoadError>() {
                Ok(load_error) => Err(load_error),
                Err(err) => Err(LoadError::Other(SharedString::from(err.to_string()))),
            },
        });
        (new_version_rx, loading_status_rx, connect_task)
    }
}

fn bool_label(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use acp_thread::StubAgentConnection;

    use super::*;

    #[test]
    fn connected_acp_details_include_agent_capabilities() {
        let connection = StubAgentConnection::new()
            .with_agent_id("test-agent".into())
            .with_supports_load_session(true);
        let entry = AgentConnectionEntry::Connected(AgentConnectedState {
            connection: Rc::new(connection),
        });

        let details = entry.acp_details();

        assert_eq!(details.status, AgentConnectionStatus::Connected);
        assert_eq!(
            details.agent_id.as_ref().map(|id| id.as_str()),
            Some("test-agent")
        );
        assert_eq!(details.agent_version, None);
        assert!(details.supports_load_session);
        assert!(!details.supports_close_session);
        assert!(details.supports_session_history);
        assert!(details.summary_label().contains("ACP connected"));
        assert!(
            details
                .tooltip_text()
                .contains("Load/resume/close: yes/no/no")
        );
    }

    #[test]
    fn disconnected_acp_details_are_explicit() {
        let entry = AgentConnectionEntry::Error {
            error: LoadError::Other("nope".into()),
        };

        let details = entry.acp_details();

        assert_eq!(details.status, AgentConnectionStatus::Disconnected);
        assert_eq!(details.summary_label(), "ACP disconnected");
        assert!(details.tooltip_text().contains("ACP status: Disconnected"));
    }
}
