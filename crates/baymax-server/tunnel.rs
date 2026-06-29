use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelStatus {
    Stopped,
    Running { public_url: String },
}

#[derive(Debug, Clone)]
pub struct TunnelManager {
    local_port: u16,
    relay_url: String,
    status: TunnelStatus,
}

impl TunnelManager {
    pub fn new(local_port: u16, relay_url: impl Into<String>) -> Self {
        Self {
            local_port,
            relay_url: relay_url.into(),
            status: TunnelStatus::Stopped,
        }
    }

    pub fn start(&mut self) -> String {
        let public_url = format!(
            "{}/{}",
            self.relay_url.trim_end_matches('/'),
            self.local_port
        );
        self.status = TunnelStatus::Running {
            public_url: public_url.clone(),
        };
        public_url
    }

    pub fn stop(&mut self) {
        self.status = TunnelStatus::Stopped;
    }

    pub fn status(&self) -> TunnelStatus {
        self.status.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tunnel_reports_started_url() {
        let mut manager = TunnelManager::new(8443, "https://relay.example");

        let url = manager.start();

        assert_eq!(url, "https://relay.example/8443");
        assert!(matches!(manager.status(), TunnelStatus::Running { .. }));
    }
}
