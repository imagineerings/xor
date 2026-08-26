use anyhow::{Result, bail};
use task::{MAX_TASK_ARTIFACT_BYTES, TaskArtifact, TaskArtifactKind};

use crate::cargo_preset::CompiledCargoPreset;

pub const CARGO_COVERAGE_ARTIFACT_PATH: &str = "target/zed-coverage.json";
pub const CARGO_COVERAGE_FAILURE_GUIDANCE: &str = "Coverage failed. Verify that cargo-llvm-cov is installed on the project host with `cargo install cargo-llvm-cov`, then inspect the task output.";

pub fn compile_coverage_plan(
    mut compiled_cargo_context: CompiledCargoPreset,
) -> Result<CompiledCargoPreset> {
    if compiled_cargo_context.task_template.command != "cargo" {
        bail!("Run with Coverage requires the Cargo task runner");
    }
    let arguments = &mut compiled_cargo_context.task_template.args;
    let subcommand_index = usize::from(
        arguments
            .first()
            .is_some_and(|argument| argument.starts_with('+')),
    );
    if arguments.get(subcommand_index).map(String::as_str) != Some("run") {
        bail!("Run with Coverage requires a Cargo run selection");
    }

    let original_arguments = std::mem::take(arguments);
    let mut coverage_arguments = Vec::with_capacity(original_arguments.len() + 4);
    let mut original_arguments = original_arguments.into_iter();
    if subcommand_index == 1
        && let Some(toolchain) = original_arguments.next()
    {
        coverage_arguments.push(toolchain);
    }
    let _run = original_arguments.next();
    coverage_arguments.extend([
        "llvm-cov".to_string(),
        "--json".to_string(),
        "--output-path".to_string(),
        CARGO_COVERAGE_ARTIFACT_PATH.to_string(),
        "run".to_string(),
    ]);
    coverage_arguments.extend(original_arguments);

    compiled_cargo_context.task_template.label = format!(
        "Run with Coverage ({})",
        compiled_cargo_context.task_template.label
    );
    compiled_cargo_context.task_template.args = coverage_arguments;
    compiled_cargo_context
        .task_template
        .tags
        .retain(|tag| !tag.starts_with("cargo-"));
    compiled_cargo_context
        .task_template
        .tags
        .extend(["cargo-coverage".to_string(), "external-tool".to_string()]);
    compiled_cargo_context.task_template.artifact = Some(TaskArtifact {
        path: CARGO_COVERAGE_ARTIFACT_PATH.to_string(),
        kind: TaskArtifactKind::Data,
        max_bytes: MAX_TASK_ARTIFACT_BYTES,
    });
    Ok(compiled_cargo_context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_preset::{
        CargoCompileContext, CargoPreset, CargoPresetScope, CargoSubcommand, CargoTargetSelector,
        CargoWorkingDirectoryPolicy, compile_preset,
    };
    use collections::HashMap;

    #[test]
    fn cargo_coverage_preserves_compiled_context_and_declares_exact_host_artifact() {
        let mut preset = CargoPreset::ephemeral_default(CargoSubcommand::Run);
        preset.label = "release app".to_string();
        preset.scope = CargoPresetScope::Package;
        preset.package = Some("app".to_string());
        preset.target = Some(CargoTargetSelector::Binary("server".to_string()));
        preset.toolchain = Some("nightly".to_string());
        preset.features = vec!["telemetry".to_string()];
        preset.environment =
            HashMap::from_iter([("SECRET_TOKEN".to_string(), "secret".to_string())]);
        preset.working_directory = CargoWorkingDirectoryPolicy::Package;
        let compiled = compile_preset(
            &preset,
            &CargoCompileContext {
                workspace_cwd: Some("/workspace".to_string()),
                package_cwd: Some("/workspace/app".to_string()),
                ..CargoCompileContext::default()
            },
            None,
        )
        .expect("Cargo run should compile");

        let coverage = compile_coverage_plan(compiled).expect("coverage plan should compile");
        assert_eq!(coverage.task_template.command, "cargo");
        assert_eq!(
            coverage.task_template.args,
            [
                "+nightly",
                "llvm-cov",
                "--json",
                "--output-path",
                CARGO_COVERAGE_ARTIFACT_PATH,
                "run",
                "--package",
                "app",
                "--bin",
                "server",
                "--features",
                "telemetry",
            ]
        );
        assert_eq!(
            coverage.task_template.cwd.as_deref(),
            Some("/workspace/app")
        );
        assert_eq!(
            coverage
                .task_template
                .env
                .get("SECRET_TOKEN")
                .map(String::as_str),
            Some("secret")
        );
        let artifact = coverage
            .task_template
            .artifact
            .as_ref()
            .expect("coverage should declare its report");
        assert_eq!(artifact.path, CARGO_COVERAGE_ARTIFACT_PATH);
        assert_eq!(artifact.kind, TaskArtifactKind::Data);
        assert_eq!(artifact.max_bytes, MAX_TASK_ARTIFACT_BYTES);
        assert!(
            coverage
                .task_template
                .tags
                .contains(&"cargo-coverage".to_string())
        );
        assert!(!format!("{coverage:?}").contains("SECRET_TOKEN=secret"));
    }

    #[test]
    fn cargo_coverage_rejects_non_run_tasks_and_has_bounded_setup_guidance() {
        let compiled = compile_preset(
            &CargoPreset::ephemeral_default(CargoSubcommand::Test),
            &CargoCompileContext::default(),
            None,
        )
        .expect("Cargo test should compile");
        assert!(compile_coverage_plan(compiled).is_err());
        assert!(CARGO_COVERAGE_FAILURE_GUIDANCE.contains("cargo install cargo-llvm-cov"));
        assert!(CARGO_COVERAGE_FAILURE_GUIDANCE.len() < 512);
    }
}
