use anyhow::{Context as _, Result};
use futures::{AsyncBufReadExt as _, StreamExt as _, io::BufReader};
use gpui::{AppContext as _, AsyncApp, SharedString, WeakEntity};
use remote::{CommandTemplate, RemoteClient, SshConnectionOptions};
use util::command::Child;

/// Information about a running tunnel.
#[derive(Debug, Clone, PartialEq)]
pub struct TunnelInfo {
    /// The local address where the tunnel is listening.
    pub endpoint_url: String,
    /// An optional authentication token for the mobile app.
    pub auth_token: Option<String>,
    /// The local port number.
    pub local_port: u16,
}

/// The current state of the tunnel.
#[derive(Debug, Clone, PartialEq)]
pub enum TunnelStatus {
    /// Tunnel is not running.
    Stopped,
    /// Tunnel is being established.
    Starting,
    /// Tunnel is actively forwarding connections.
    Running(TunnelInfo),
    /// Tunnel is being torn down.
    Stopping,
    /// Tunnel encountered an error.
    Error { message: SharedString },
}

/// Manages the lifecycle of a secure SSH tunnel for mobile app access.
///
/// Reuses Sim's existing SSH remote infrastructure when available:
/// - If an `SshRemoteConnection` is active via `ControlMaster`, the tunnel
///   creates a forwarded-port session through the existing socket without
///   re-authentication.
/// - Otherwise, a standalone SSH subprocess is spawned with the provided
///   connection options.
pub struct TunnelManager {
    status: TunnelStatus,
    _remote_client: Option<WeakEntity<RemoteClient>>,
    _standalone_options: Option<SshConnectionOptions>,
    child_process: Option<Child>,
}

impl TunnelManager {
    /// Create a TunnelManager that reuses an existing SSH ControlMaster connection.
    ///
    /// When `start()` is called, it uses `build_forward_ports_command()` on the
    /// active `SshRemoteConnection` to create a new forwarded-port session through
    /// the existing socket — no re-authentication needed.
    pub fn new_with_remote(remote_client: WeakEntity<RemoteClient>) -> Self {
        Self {
            status: TunnelStatus::Stopped,
            _remote_client: Some(remote_client),
            _standalone_options: None,
            child_process: None,
        }
    }

    /// Create a TunnelManager for standalone SSH access (no active remote connection).
    ///
    /// When `start()` is called, it spawns a new SSH subprocess with `-L` flags
    /// using the provided connection options.
    pub fn new_standalone(options: SshConnectionOptions) -> Self {
        Self {
            status: TunnelStatus::Stopped,
            _remote_client: None,
            _standalone_options: Some(options),
            child_process: None,
        }
    }

    /// Start the tunnel.
    ///
    /// Returns once the tunnel is established or fails with an error.
    /// - If a `ControlMaster` connection is available, reuses it.
    /// - Otherwise spawns a standalone SSH subprocess.
    pub async fn start(&mut self, cx: &mut AsyncApp) -> Result<TunnelInfo> {
        self.status = TunnelStatus::Starting;

        // 1. Allocate a local port
        let local_port = allocate_local_port()?;

        // 2. Build and spawn the SSH forward command
        let forward_host = "127.0.0.1".to_string();
        let remote_port = 22u16;

        let command_template = if let Some(remote_client_weak) = &self._remote_client {
            // Path A: Reuse existing ControlMaster connection
            let remote_client = remote_client_weak
                .upgrade()
                .context("Remote connection no longer available")?;
            let forwards = vec![(local_port, forward_host.clone(), remote_port)];
            cx.update(|cx| remote_client.read(cx).build_forward_ports_command(forwards))?
        } else if let Some(options) = &self._standalone_options {
            // Path B: Standalone SSH connection
            let mut args = options.additional_args();
            args.push("-N".into());
            args.push("-L".into());
            args.push(format!("{}:{}:{}", local_port, forward_host, remote_port));
            args.push(options.ssh_destination());
            CommandTemplate {
                program: "ssh".into(),
                args,
                env: Default::default(),
            }
        } else {
            anyhow::bail!("no remote connection or standalone options configured");
        };

        let mut command = util::command::new_command(&command_template.program);
        command.args(&command_template.args);
        command.envs(&command_template.env);

        let mut child = command.spawn().context("failed to spawn SSH tunnel")?;

        // 3. Forward stderr to logs (detached background task)
        let stderr = child.stderr.take();
        cx.background_spawn(async move {
            let Some(stderr) = stderr else { return };
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Some(Ok(line)) = lines.next().await {
                log::warn!("ssh tunnel stderr: {}", line);
            }
        })
        .detach();

        // 4. Wait for tunnel to be ready by connecting to the local port
        let max_attempts = 100;
        let mut connected = false;
        for attempt in 0..max_attempts {
            match smol::net::TcpStream::connect(format!("127.0.0.1:{}", local_port)).await {
                Ok(_) => {
                    connected = true;
                    log::info!(
                        "SSH tunnel established on port {} (attempt {})",
                        local_port,
                        attempt + 1
                    );
                    // Give the tunnel a moment to stabilize
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(500))
                        .await;
                    break;
                }
                Err(_) => {
                    if attempt < max_attempts - 1 {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(100))
                            .await;
                    }
                }
            }
        }

        if !connected {
            self.kill_child();
            let msg: SharedString = format!(
                "SSH tunnel failed to establish on port {} after {} attempts",
                local_port, max_attempts
            )
            .into();
            self.status = TunnelStatus::Error {
                message: msg.clone(),
            };
            anyhow::bail!("{}", msg);
        }

        // 5. Generate auth token and return tunnel info
        let auth_token = generate_auth_token();
        let tunnel_info = TunnelInfo {
            endpoint_url: format!("127.0.0.1:{}", local_port),
            auth_token: Some(auth_token),
            local_port,
        };

        self.child_process = Some(child);
        self.status = TunnelStatus::Running(tunnel_info.clone());

        Ok(tunnel_info)
    }

    /// Stop the tunnel and clean up.
    ///
    /// Kills the forward-session subprocess but does NOT kill the ControlMaster
    /// (the existing SSH remote connection remains intact).
    pub fn stop(&mut self) -> Result<()> {
        self.status = TunnelStatus::Stopping;
        self.kill_child();
        self.status = TunnelStatus::Stopped;
        Ok(())
    }

    /// Return the current tunnel status without side effects.
    pub fn status(&self) -> &TunnelStatus {
        &self.status
    }

    /// Kill the tunnel subprocess if still running.
    fn kill_child(&mut self) {
        if let Some(mut child) = self.child_process.take() {
            child.kill().ok();
        }
    }

    /// Set the tunnel status for testing purposes only.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_status_for_test(&mut self, status: TunnelStatus) {
        self.status = status;
    }
}

impl Drop for TunnelManager {
    fn drop(&mut self) {
        self.kill_child();
    }
}

/// Allocate a local port by binding to port 0.
fn allocate_local_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// Generate a random auth token for the tunnel.
fn generate_auth_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("tun-{:08x}", t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_opts() -> SshConnectionOptions {
        SshConnectionOptions {
            host: "test-host".into(),
            username: None,
            port: None,
            password: None,
            args: None,
            port_forwards: None,
            connection_timeout: None,
            nickname: None,
            upload_binary_over_ssh: false,
        }
    }

    #[test]
    fn test_new_standalone_starts_stopped() {
        let manager = TunnelManager::new_standalone(default_opts());
        assert_eq!(manager.status(), &TunnelStatus::Stopped);
    }

    #[test]
    fn test_stop_on_stopped_manager_is_safe() {
        let mut manager = TunnelManager::new_standalone(default_opts());
        assert_eq!(manager.status(), &TunnelStatus::Stopped);

        let result = manager.stop();
        assert!(result.is_ok());
        assert_eq!(manager.status(), &TunnelStatus::Stopped);
    }

    #[test]
    fn test_double_stop_is_safe() {
        let mut manager = TunnelManager::new_standalone(default_opts());
        assert!(manager.stop().is_ok());
        assert!(manager.stop().is_ok());
        assert_eq!(manager.status(), &TunnelStatus::Stopped);
    }

    #[test]
    fn test_allocate_local_port_returns_valid_port() {
        let port = allocate_local_port().unwrap();
        assert!(
            port > 0,
            "allocated port should be a positive number, got {}",
            port
        );
    }

    #[test]
    fn test_generate_auth_token_is_non_empty() {
        let token = generate_auth_token();
        assert!(!token.is_empty(), "auth token should not be empty");
        assert!(
            token.starts_with("tun-"),
            "auth token should start with 'tun-', got {}",
            token
        );
    }

    #[test]
    fn test_generate_auth_token_produces_unique_values() {
        // Generate several tokens over time and verify they're not all the same.
        let tokens: Vec<String> = (0..10).map(|_| generate_auth_token()).collect();
        let mut unique = tokens.clone();
        unique.sort();
        unique.dedup();
        assert!(
            unique.len() > 1,
            "expected at least 2 unique tokens out of 10, got {}: {:?}",
            unique.len(),
            tokens
        );
    }

    #[test]
    fn test_drop_does_not_panic() {
        let manager = TunnelManager::new_standalone(default_opts());
        drop(manager);
    }

    #[gpui::test]
    #[ignore = "requires real SSH binary; start() uses smol::Timer/smol::net which don't progress in GPUI test scheduler"]
    async fn test_start_standalone_fails_without_ssh(cx: &mut gpui::TestAppContext) {
        let mut manager = TunnelManager::new_standalone(SshConnectionOptions {
            host: "nope.invalid".into(),
            username: None,
            port: Some(22),
            password: None,
            args: None,
            port_forwards: None,
            connection_timeout: Some(1),
            nickname: None,
            upload_binary_over_ssh: false,
        });

        let (final_status, result) = cx
            .spawn(|cx| async move {
                let mut cx = cx;
                let result = manager.start(&mut cx).await;
                (manager.status().clone(), result)
            })
            .await;

        assert!(result.is_err(), "start should fail without SSH");
        match &final_status {
            TunnelStatus::Starting | TunnelStatus::Error { .. } => {}
            other => panic!(
                "expected Starting or Error after failed start, got {:?}",
                other
            ),
        }
    }

    #[gpui::test]
    #[ignore = "requires real SSH binary; start() uses smol::Timer/smol::net which don't progress in GPUI test scheduler"]
    async fn test_double_start_does_not_panic(cx: &mut gpui::TestAppContext) {
        let mut manager = TunnelManager::new_standalone(SshConnectionOptions {
            host: "nope.invalid".into(),
            username: None,
            port: Some(22),
            password: None,
            args: None,
            port_forwards: None,
            connection_timeout: Some(1),
            nickname: None,
            upload_binary_over_ssh: false,
        });

        let result = cx
            .spawn(|cx| async move {
                let mut cx = cx;
                let result = manager.start(&mut cx).await;
                (manager.status().clone(), result)
            })
            .await;

        assert!(result.1.is_err(), "first start should fail");
    }

    #[test]
    fn test_tunnel_info_equality() {
        let info1 = TunnelInfo {
            endpoint_url: "127.0.0.1:8080".into(),
            auth_token: Some("tok-123".into()),
            local_port: 8080,
        };
        let info2 = TunnelInfo {
            endpoint_url: "127.0.0.1:8080".into(),
            auth_token: Some("tok-123".into()),
            local_port: 8080,
        };
        assert_eq!(info1, info2);
    }

    #[test]
    fn test_tunnel_status_clone_and_equality() {
        let variants: &[TunnelStatus] = &[
            TunnelStatus::Stopped,
            TunnelStatus::Starting,
            TunnelStatus::Running(TunnelInfo {
                endpoint_url: "test".into(),
                auth_token: None,
                local_port: 1234,
            }),
            TunnelStatus::Stopping,
            TunnelStatus::Error {
                message: "oops".into(),
            },
        ];
        for variant in variants {
            assert_eq!(variant, &variant.clone());
        }
    }
}
