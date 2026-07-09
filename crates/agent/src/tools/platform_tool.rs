use std::{path::PathBuf, process::Command, sync::Arc};

use agent_client_protocol::schema as acp;
use agent_settings::AgentSettings;
use anyhow::{Context as _, Result, bail};
use gpui::{App, AppContext as _, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings::Settings as _;
use util::markdown::MarkdownInlineCode;

use crate::{
    AgentTool, ToolCallEventStream, ToolInput, ToolPermissionContext, ToolPermissionDecision,
    authorize_with_sensitive_settings, decide_permission_for_path,
};

/// Run a small set of platform-specific desktop operations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlatformToolInput {
    /// Operation to run: "platform_info", "open_path", or "reveal_path".
    pub operation: String,
    /// Path for open_path or reveal_path.
    #[serde(default)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformToolOutput {
    pub success: bool,
    pub operation: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
}

impl From<PlatformToolOutput> for LanguageModelToolResultContent {
    fn from(output: PlatformToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| format!("failed to serialize platform tool output: {error}"))
            .into()
    }
}

pub struct PlatformTool;

impl AgentTool for PlatformTool {
    type Input = PlatformToolInput;
    type Output = PlatformToolOutput;

    const NAME: &'static str = "platform_tool";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(input) => match input.path {
                Some(path) => {
                    let path = path.display().to_string();
                    format!("Platform {} {}", input.operation, MarkdownInlineCode(&path)).into()
                }
                None => format!("Platform {}", input.operation).into(),
            },
            Err(_) => "Run platform operation".into(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| PlatformToolOutput {
                success: false,
                operation: "unknown".to_string(),
                message: format!("Failed to read platform tool input: {error}"),
                os: None,
                family: None,
                arch: None,
            })?;
            if let Some(path) = input.path.as_ref() {
                authorize_path(Self::NAME, path, "Use platform tool", &event_stream, cx)
                    .await
                    .map_err(|error| PlatformToolOutput {
                        success: false,
                        operation: input.operation.clone(),
                        message: error.to_string(),
                        os: None,
                        family: None,
                        arch: None,
                    })?;
            }
            cx.background_spawn(async move { run_platform_tool(input) })
                .await
                .map_err(|error| PlatformToolOutput {
                    success: false,
                    operation: "unknown".to_string(),
                    message: error.to_string(),
                    os: None,
                    family: None,
                    arch: None,
                })
        })
    }
}

fn authorize_path(
    tool_name: &str,
    path: &PathBuf,
    title: &str,
    event_stream: &ToolCallEventStream,
    cx: &mut gpui::AsyncApp,
) -> Task<Result<()>> {
    let path = path.display().to_string();
    cx.update(|cx| {
        match decide_permission_for_path(tool_name, &path, AgentSettings::get_global(cx)) {
            ToolPermissionDecision::Allow => Task::ready(Ok(())),
            ToolPermissionDecision::Deny(reason) => Task::ready(Err(anyhow::anyhow!(reason))),
            ToolPermissionDecision::Confirm => {
                let context = ToolPermissionContext::new(tool_name, vec![path]);
                authorize_with_sensitive_settings(None, context, title, event_stream, cx)
            }
        }
    })
}

fn run_platform_tool(input: PlatformToolInput) -> Result<PlatformToolOutput> {
    match input.operation.as_str() {
        "platform_info" => Ok(PlatformToolOutput {
            success: true,
            operation: input.operation,
            message: "Read platform information".to_string(),
            os: Some(std::env::consts::OS.to_string()),
            family: Some(std::env::consts::FAMILY.to_string()),
            arch: Some(std::env::consts::ARCH.to_string()),
        }),
        "open_path" => {
            let path = input.path.context("open_path requires path")?;
            run_platform_command(open_command(&path)?)?;
            Ok(PlatformToolOutput {
                success: true,
                operation: input.operation,
                message: format!("Opened {}", path.display()),
                os: None,
                family: None,
                arch: None,
            })
        }
        "reveal_path" => {
            let path = input.path.context("reveal_path requires path")?;
            run_platform_command(reveal_command(&path)?)?;
            Ok(PlatformToolOutput {
                success: true,
                operation: input.operation,
                message: format!("Revealed {}", path.display()),
                os: None,
                family: None,
                arch: None,
            })
        }
        operation => bail!("unsupported platform operation: {operation}"),
    }
}

fn open_command(path: &PathBuf) -> Result<Command> {
    if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(path);
        Ok(command)
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(path);
        Ok(command)
    } else if cfg!(target_os = "linux") {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        Ok(command)
    } else {
        bail!("open_path is unsupported on this platform")
    }
}

fn reveal_command(path: &PathBuf) -> Result<Command> {
    if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg("-R").arg(path);
        Ok(command)
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(format!("/select,{}", path.display()));
        Ok(command)
    } else if cfg!(target_os = "linux") {
        let directory = if path.is_dir() {
            path.clone()
        } else {
            path.parent()
                .context("reveal_path requires a path with a parent directory")?
                .to_path_buf()
        };
        let mut command = Command::new("xdg-open");
        command.arg(directory);
        Ok(command)
    } else {
        bail!("reveal_path is unsupported on this platform")
    }
}

fn run_platform_command(mut command: Command) -> Result<()> {
    #[allow(
        clippy::disallowed_methods,
        reason = "platform commands run inside the tool background task, not on the foreground thread"
    )]
    let output = command
        .output()
        .context("failed to start platform command")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("platform command failed: {stderr}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_info_reports_current_target() {
        let output = run_platform_tool(PlatformToolInput {
            operation: "platform_info".to_string(),
            path: None,
        })
        .unwrap();
        assert_eq!(output.os.as_deref(), Some(std::env::consts::OS));
        assert!(output.success);
    }
}
