#![allow(dead_code)]

use anyhow::{Context, Result};
use clap::Parser;
use gh_workflow::Workflow;
use std::fs;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;

use crate::tasks::workflow_checks::{self};

mod deploy_docs;
mod extensions;
mod release;
mod run_bundling;
mod run_tests;
mod runners;
mod steps;
mod vars;

#[derive(Clone)]
pub(crate) struct GitSha(String);

impl AsRef<str> for GitSha {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[allow(
    clippy::disallowed_methods,
    reason = "This runs only in a CLI environment"
)]
fn parse_ref(value: &str) -> Result<GitSha, String> {
    const GIT_SHA_LENGTH: usize = 40;
    (value.len() == GIT_SHA_LENGTH)
        .then_some(value)
        .ok_or_else(|| {
            format!(
                "Git SHA has wrong length! \
                Only SHAs with a full length of {GIT_SHA_LENGTH} are supported, found {len} characters.",
                len = value.len()
            )
        })
        .and_then(|value| {
            let mut tmp = [0; 4];
            value
                .chars()
                .all(|char| u16::from_str_radix(char.encode_utf8(&mut tmp), 16).is_ok()).then_some(value)
                .ok_or_else(|| "Not a valid Git SHA".to_owned())
        })
        .and_then(|sha| {
           std::process::Command::new("git")
               .args([
                   "rev-parse",
                   "--quiet",
                   "--verify",
                   &format!("{sha}^{{commit}}")
               ])
               .output()
               .map_err(|_| "Failed to spawn Git command to verify SHA".to_owned())
               .and_then(|output|
                   output
                       .status.success()
                       .then_some(sha)
                       .ok_or_else(|| format!("SHA {sha} is not a valid Git SHA within this repository!")))
        }).map(|sha| GitSha(sha.to_owned()))
}

#[derive(Parser)]
pub(crate) struct GenerateWorkflowArgs {
    #[arg(value_parser = parse_ref)]
    /// The Git SHA to use when invoking this
    pub(crate) sha: Option<GitSha>,
}

enum WorkflowSource {
    Contextless(fn() -> Workflow),
    WithContext(fn(&GenerateWorkflowArgs) -> Workflow),
}

struct WorkflowFile {
    source: WorkflowSource,
    r#type: WorkflowType,
}

const ARCHIVED_SIM_WORKFLOW_FILENAMES: &[&str] = &[
    "after_release.yml",
    "autofix_pr.yml",
    "bump_sim_version.yml",
    "bump_patch_version.yml",
    "cherry_pick.yml",
    "compliance_check.yml",
    "danger.yml",
    "deploy_collab.yml",
    "deploy_docs.yml",
    "deploy_nightly_docs.yml",
    "extension_auto_bump.yml",
    "extension_bump.yml",
    "extension_tests.yml",
    "extension_workflow_rollout.yml",
    "nix_build.yml",
    "publish_extension_cli.yml",
    "release.yml",
    "release_nightly.yml",
    "run_bundling.yml",
    "run_tests.yml",
];

impl WorkflowFile {
    fn sim(f: fn() -> Workflow) -> WorkflowFile {
        WorkflowFile {
            source: WorkflowSource::Contextless(f),
            r#type: WorkflowType::Sim,
        }
    }

    fn extension(f: fn(&GenerateWorkflowArgs) -> Workflow) -> WorkflowFile {
        WorkflowFile {
            source: WorkflowSource::WithContext(f),
            r#type: WorkflowType::ExtensionCi,
        }
    }

    fn extension_shared(f: fn(&GenerateWorkflowArgs) -> Workflow) -> WorkflowFile {
        WorkflowFile {
            source: WorkflowSource::WithContext(f),
            r#type: WorkflowType::ExtensionsShared,
        }
    }

    fn generate_file(&self, workflow_args: &GenerateWorkflowArgs) -> Result<()> {
        let workflow = match &self.source {
            WorkflowSource::Contextless(f) => f(),
            WorkflowSource::WithContext(f) => f(workflow_args),
        };
        let workflow_folder = self.r#type.folder_path();

        fs::create_dir_all(&workflow_folder).with_context(|| {
            format!("Failed to create directory: {}", workflow_folder.display())
        })?;

        let workflow_name = workflow
            .name
            .as_ref()
            .expect("Workflow must have a name at this point");
        let filename = format!(
            "{}.yml",
            workflow_name.rsplit("::").next().unwrap_or(workflow_name)
        );

        if self.r#type.should_skip_archived_workflow(&filename) {
            println!("Skipping archived workflow: {filename}");
            return Ok(());
        }

        let workflow_path = workflow_folder.join(filename);

        let content = workflow
            .to_string()
            .map_err(|e| anyhow::anyhow!("{:?}: {:?}", workflow_path, e))?;

        let disclaimer = self.r#type.disclaimer(workflow_name);

        let content = [disclaimer, content].join("\n");
        fs::write(&workflow_path, content).map_err(Into::into)
    }
}

#[derive(PartialEq, Eq, strum::EnumIter)]
pub enum WorkflowType {
    /// Workflows living in the Sim repository
    Sim,
    /// Workflows living in the `sim-extensions/workflows` repository that are
    /// required workflows for PRs to the extension organization
    ExtensionCi,
    /// Workflows living in each of the extensions to perform checks and version
    /// bumps until a better, more centralisim system for that is in place.
    ExtensionsShared,
}

impl WorkflowType {
    const PREAMBLE: &str = "# Generated from xtask::workflows::";

    fn disclaimer(&self, workflow_name: &str) -> String {
        format!(
            concat!(
                "{preamble}{workflow_name}{external_disclaimer}\n",
                "# Rebuild with `cargo xtask workflows`.",
            ),
            preamble = Self::PREAMBLE,
            workflow_name = workflow_name,
            external_disclaimer = (*self != WorkflowType::Sim)
                .then_some(" within the Sim repository.")
                .unwrap_or_default(),
        )
    }

    pub fn folder_path(&self) -> PathBuf {
        match self {
            WorkflowType::Sim => PathBuf::from(".github/workflows"),
            WorkflowType::ExtensionCi => PathBuf::from("extensions/workflows"),
            WorkflowType::ExtensionsShared => PathBuf::from("extensions/workflows/shared"),
        }
    }

    fn should_skip_archived_workflow(&self, filename: &str) -> bool {
        *self == WorkflowType::Sim && ARCHIVED_SIM_WORKFLOW_FILENAMES.contains(&filename)
    }

    fn remove_generated_workflows() -> Result<()> {
        for workflow_type in Self::iter() {
            for path in fs::read_dir(workflow_type.folder_path())? {
                let entry = path?;
                if !entry.file_type().is_ok_and(|file_type| file_type.is_file()) {
                    continue;
                }

                let path = entry.path();
                if fs::read_to_string(&path)
                    .is_ok_and(|content| content.starts_with(Self::PREAMBLE))
                {
                    fs::remove_file(path)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::WorkflowType;

    #[test]
    fn skips_archived_sim_workflows() {
        assert!(WorkflowType::Sim.should_skip_archived_workflow("run_bundling.yml"));
        assert!(WorkflowType::Sim.should_skip_archived_workflow("release.yml"));
        assert!(WorkflowType::Sim.should_skip_archived_workflow("run_tests.yml"));
    }

    #[test]
    fn does_not_skip_extension_repository_workflows() {
        assert!(!WorkflowType::ExtensionCi.should_skip_archived_workflow("run_tests.yml"));
        assert!(!WorkflowType::ExtensionsShared.should_skip_archived_workflow("bump_version.yml"));
    }

    #[test]
    fn does_not_skip_unarchived_sim_workflows() {
        assert!(!WorkflowType::Sim.should_skip_archived_workflow("mobile_android_ci.yml"));
    }
}

pub fn run_workflows(args: GenerateWorkflowArgs) -> Result<()> {
    if !Path::new("crates/sim/").is_dir() {
        anyhow::bail!("xtask workflows must be ran from the project root");
    }

    // Remove all previously generated workflows to ensure these do not become stale.
    WorkflowType::remove_generated_workflows()?;

    let workflows = [
        // Core: release.yml and run_tests.yml are now hand-written minimal versions (not generated)
        /* workflows used for CI/CD in extension repositories */
        WorkflowFile::extension(extensions::run_tests::run_tests),
        WorkflowFile::extension_shared(extensions::bump_version::bump_version),
    ];

    for workflow_file in workflows {
        workflow_file.generate_file(&args)?;
    }

    workflow_checks::validate(Default::default())
}
