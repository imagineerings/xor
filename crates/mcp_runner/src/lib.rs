use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    process::ExitStatus,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct McpServerCommand {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<(String, String)>,
    pub working_directory: Option<PathBuf>,
}

impl McpServerCommand {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            working_directory: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StdioTransportConfig {
    pub command: McpServerCommand,
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout: Duration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum HealthCheck {
    ProcessAlive,
    Initialized,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CrashRecoveryPolicy {
    pub max_restarts: usize,
    #[serde(default = "default_restart_backoff")]
    pub restart_backoff: Duration,
}

impl Default for CrashRecoveryPolicy {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            restart_backoff: default_restart_backoff(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum McpServerStatus {
    Stopped,
    Starting,
    Running,
    Unhealthy(String),
    Crashed(Option<ExitStatus>),
}

#[derive(Clone, Debug)]
pub struct McpServerRunnerConfig {
    pub transport: StdioTransportConfig,
    pub health_check: HealthCheck,
    pub crash_recovery: CrashRecoveryPolicy,
}

pub trait McpServerProcess {
    fn start(&mut self, command: &McpServerCommand) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn is_running(&self) -> bool;
    fn exit_status(&self) -> Option<ExitStatus>;
    fn initialized(&self) -> bool;
}

#[derive(Debug)]
pub struct McpServerRunner<P> {
    config: McpServerRunnerConfig,
    process: P,
    status: McpServerStatus,
    restart_count: usize,
    last_started_at: Option<Instant>,
}

impl<P: McpServerProcess> McpServerRunner<P> {
    pub fn new(config: McpServerRunnerConfig, process: P) -> Self {
        Self {
            config,
            process,
            status: McpServerStatus::Stopped,
            restart_count: 0,
            last_started_at: None,
        }
    }

    pub fn status(&self) -> &McpServerStatus {
        &self.status
    }

    pub fn restart_count(&self) -> usize {
        self.restart_count
    }

    pub fn start(&mut self) -> Result<()> {
        self.status = McpServerStatus::Starting;
        self.process.start(&self.config.transport.command)?;
        self.last_started_at = Some(Instant::now());
        self.status = McpServerStatus::Running;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.process.stop()?;
        self.status = McpServerStatus::Stopped;
        Ok(())
    }

    pub fn check_health(&mut self) -> McpServerStatus {
        if !self.process.is_running() {
            self.status = McpServerStatus::Crashed(self.process.exit_status());
            return self.status.clone();
        }

        match self.config.health_check {
            HealthCheck::ProcessAlive => {
                self.status = McpServerStatus::Running;
            }
            HealthCheck::Initialized => {
                if self.process.initialized() {
                    self.status = McpServerStatus::Running;
                } else if self.last_started_at.is_some_and(|started_at| {
                    started_at.elapsed() > self.config.transport.startup_timeout
                }) {
                    self.status = McpServerStatus::Unhealthy(
                        "MCP server did not initialize before startup timeout".to_string(),
                    );
                } else {
                    self.status = McpServerStatus::Starting;
                }
            }
        }

        self.status.clone()
    }

    pub fn recover_if_crashed(&mut self) -> Result<bool> {
        if !matches!(self.check_health(), McpServerStatus::Crashed(_)) {
            return Ok(false);
        }

        if self.restart_count >= self.config.crash_recovery.max_restarts {
            return Err(anyhow!(
                "MCP server crashed after {} restart attempt(s)",
                self.restart_count
            ));
        }

        self.restart_count += 1;
        self.start()?;
        Ok(true)
    }

    pub fn into_process(self) -> P {
        self.process
    }
}

fn default_startup_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_restart_backoff() -> Duration {
    Duration::from_millis(250)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code)
    }

    #[cfg(windows)]
    fn exit_status(code: u32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code)
    }

    #[derive(Debug, Default)]
    struct FakeProcess {
        running: bool,
        initialized: bool,
        starts: usize,
        stops: usize,
        exit_status: Option<ExitStatus>,
    }

    impl FakeProcess {
        fn crash(&mut self) {
            self.running = false;
            self.exit_status = Some(exit_status(9));
        }
    }

    impl McpServerProcess for FakeProcess {
        fn start(&mut self, _command: &McpServerCommand) -> Result<()> {
            self.running = true;
            self.initialized = true;
            self.exit_status = None;
            self.starts += 1;
            Ok(())
        }

        fn stop(&mut self) -> Result<()> {
            self.running = false;
            self.stops += 1;
            Ok(())
        }

        fn is_running(&self) -> bool {
            self.running
        }

        fn exit_status(&self) -> Option<ExitStatus> {
            self.exit_status
        }

        fn initialized(&self) -> bool {
            self.initialized
        }
    }

    fn runner(process: FakeProcess) -> McpServerRunner<FakeProcess> {
        McpServerRunner::new(
            McpServerRunnerConfig {
                transport: StdioTransportConfig {
                    command: McpServerCommand::new("memory-mcp"),
                    startup_timeout: Duration::from_secs(1),
                },
                health_check: HealthCheck::Initialized,
                crash_recovery: CrashRecoveryPolicy {
                    max_restarts: 1,
                    restart_backoff: Duration::from_millis(1),
                },
            },
            process,
        )
    }

    #[test]
    fn start_and_stop_manage_lifecycle_state() {
        let mut runner = runner(FakeProcess::default());

        runner.start().expect("start server");
        assert_eq!(runner.status(), &McpServerStatus::Running);

        runner.stop().expect("stop server");
        assert_eq!(runner.status(), &McpServerStatus::Stopped);

        let process = runner.into_process();
        assert_eq!(process.starts, 1);
        assert_eq!(process.stops, 1);
    }

    #[test]
    fn health_check_marks_crashed_process() {
        let mut process = FakeProcess::default();
        process.crash();
        let mut runner = runner(process);

        let status = runner.check_health();

        assert!(matches!(status, McpServerStatus::Crashed(Some(_))));
    }

    #[test]
    fn crashed_process_restarts_within_policy() {
        let mut runner = runner(FakeProcess::default());
        runner.start().expect("start server");
        runner.process.crash();

        let recovered = runner.recover_if_crashed().expect("recover crash");

        assert!(recovered);
        assert_eq!(runner.restart_count(), 1);
        assert_eq!(runner.status(), &McpServerStatus::Running);
    }

    #[test]
    fn crash_recovery_reports_exhausted_policy() {
        let mut runner = runner(FakeProcess::default());
        runner.start().expect("start server");
        runner.process.crash();
        runner.recover_if_crashed().expect("first recovery");
        runner.process.crash();

        let error = runner.recover_if_crashed().expect_err("recovery exhausted");

        assert!(error.to_string().contains("after 1 restart"));
    }
}
