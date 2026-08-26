use anyhow::{Result, bail};
use collections::HashMap;
use task::TaskArtifact;

use crate::cargo_preset::CompiledCargoPreset;

const MAX_PROFILE_COMMAND_ITEMS: usize = 256;
const MAX_PROFILE_COMMAND_TEXT_BYTES: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalProfileCommand {
    pub command: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
}

pub fn compile_profile_plan(
    mut compiled_cargo_context: CompiledCargoPreset,
    external_command: ExternalProfileCommand,
    artifact: TaskArtifact,
) -> Result<CompiledCargoPreset> {
    validate_external_command(&external_command)?;
    if !artifact.path.contains("$ZED_") {
        artifact.with_resolved_path(artifact.path.clone())?;
    }

    compiled_cargo_context.task_template.label =
        format!("Profile {}", compiled_cargo_context.task_template.label);
    compiled_cargo_context.task_template.command = external_command.command;
    compiled_cargo_context.task_template.args = external_command.arguments;
    compiled_cargo_context
        .task_template
        .env
        .extend(external_command.environment);
    compiled_cargo_context
        .task_template
        .tags
        .retain(|tag| !tag.starts_with("cargo-"));
    compiled_cargo_context
        .task_template
        .tags
        .extend(["cargo-profile".to_string(), "external-tool".to_string()]);
    compiled_cargo_context.task_template.artifact = Some(artifact);
    Ok(compiled_cargo_context)
}

fn validate_external_command(command: &ExternalProfileCommand) -> Result<()> {
    if command.command.trim().is_empty()
        || command.command.len() > MAX_PROFILE_COMMAND_TEXT_BYTES
        || command.arguments.len() > MAX_PROFILE_COMMAND_ITEMS
        || command.environment.len() > MAX_PROFILE_COMMAND_ITEMS
    {
        bail!("external profile command is empty or exceeds the supported bounds");
    }
    if command
        .arguments
        .iter()
        .chain(command.environment.keys())
        .chain(command.environment.values())
        .any(|value| value.len() > MAX_PROFILE_COMMAND_TEXT_BYTES)
    {
        bail!("external profile command field exceeds the supported length");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_preset::{
        CargoCompileContext, CargoPreset, CargoPresetScope, CargoSubcommand,
        CargoWorkingDirectoryPolicy, compile_preset,
    };
    use task::{TaskArtifactKind, TaskContext};

    #[test]
    fn cargo_profile_plan_uses_an_explicit_external_task_and_declared_artifact() {
        let compiled = compile_preset(
            &CargoPreset {
                label: "release binary".to_string(),
                subcommand: CargoSubcommand::Run,
                scope: CargoPresetScope::Package,
                package: Some("app".to_string()),
                working_directory: CargoWorkingDirectoryPolicy::Package,
                ..CargoPreset::ephemeral_default(CargoSubcommand::Run)
            },
            &CargoCompileContext {
                workspace_cwd: Some("/workspace".to_string()),
                package_cwd: Some("/workspace/app".to_string()),
                ..CargoCompileContext::default()
            },
            None,
        )
        .expect("Cargo context should compile");
        let plan = compile_profile_plan(
            compiled,
            ExternalProfileCommand {
                command: "my-profiler".to_string(),
                arguments: vec![
                    "record".to_string(),
                    "--output".to_string(),
                    "target/profile.svg".to_string(),
                    "cargo".to_string(),
                    "run".to_string(),
                    "--package".to_string(),
                    "app".to_string(),
                ],
                environment: HashMap::from_iter([(
                    "PROFILE_MODE".to_string(),
                    "sampling".to_string(),
                )]),
            },
            TaskArtifact {
                path: "target/profile.svg".to_string(),
                kind: TaskArtifactKind::Svg,
                max_bytes: 1024 * 1024,
            },
        )
        .expect("explicit external profile plan should compile");

        assert_eq!(plan.task_template.command, "my-profiler");
        assert_eq!(plan.task_template.cwd.as_deref(), Some("/workspace/app"));
        assert_eq!(
            plan.task_template
                .artifact
                .as_ref()
                .map(|artifact| artifact.path.as_str()),
            Some("target/profile.svg")
        );
        assert!(
            plan.task_template
                .tags
                .contains(&"external-tool".to_string())
        );
        let resolved = plan
            .task_template
            .resolve_task("cargo-profile", &TaskContext::default())
            .expect("profile task should resolve without a new runner");
        assert_eq!(
            resolved
                .resolved_artifact()
                .map(|artifact| artifact.path.as_str()),
            Some("target/profile.svg")
        );
    }

    #[test]
    fn cargo_profile_plan_rejects_unsafe_or_oversized_declarations() {
        let compiled = compile_preset(
            &CargoPreset::ephemeral_default(CargoSubcommand::Build),
            &CargoCompileContext::default(),
            None,
        )
        .expect("default Cargo context should compile");
        let command = ExternalProfileCommand {
            command: "profiler".to_string(),
            arguments: Vec::new(),
            environment: HashMap::default(),
        };
        assert!(
            compile_profile_plan(
                compiled.clone(),
                command.clone(),
                TaskArtifact {
                    path: "../private/profile.svg".to_string(),
                    kind: TaskArtifactKind::Svg,
                    max_bytes: 1024,
                },
            )
            .is_err()
        );
        assert!(
            compile_profile_plan(
                compiled,
                command,
                TaskArtifact {
                    path: "target/profile.svg".to_string(),
                    kind: TaskArtifactKind::Svg,
                    max_bytes: task::MAX_TASK_ARTIFACT_BYTES + 1,
                },
            )
            .is_err()
        );
    }
}
