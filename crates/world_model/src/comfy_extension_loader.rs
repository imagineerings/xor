use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    SimExtensionId, SimExtensionPolicy, SimExtensionPolicyDecisionKind, SimExtensionPolicyRequest,
    SimExtensionRecord,
};

pub const SIM_EXTENSION_LOADER_HOOK_RESTORED_CODE: &str =
    "world_model.extensions.loader.hook_restored";
pub const SIM_EXTENSION_LOADER_IMPORT_FAILED_CODE: &str =
    "world_model.extensions.loader.import_failed";
pub const SIM_EXTENSION_LOADER_MISSING_DEPENDENCY_CODE: &str =
    "world_model.extensions.loader.missing_dependency";

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLoadMetadata {
    pub prestartup_script: Option<String>,
    pub import_error: Option<String>,
    pub missing_dependencies: Vec<String>,
    pub protected_hook_changes: Vec<String>,
}

impl SimExtensionLoadMetadata {
    pub fn with_prestartup_script(mut self, script: impl Into<String>) -> Self {
        self.prestartup_script = Some(script.into());
        self
    }

    pub fn with_import_error(mut self, error: impl Into<String>) -> Self {
        self.import_error = Some(error.into());
        self
    }

    pub fn with_missing_dependency(mut self, dependency: impl Into<String>) -> Self {
        self.missing_dependencies.push(dependency.into());
        self
    }

    pub fn with_protected_hook_change(mut self, hook: impl Into<String>) -> Self {
        self.protected_hook_changes.push(hook.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLoadedPack {
    pub extension_id: SimExtensionId,
    pub source_path: PathBuf,
    pub policy_decision: SimExtensionPolicyDecisionKind,
    pub prestartup_script_ran: bool,
    pub restored_hooks: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionSkippedPack {
    pub extension_id: SimExtensionId,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLoaderDiagnostic {
    pub code: String,
    pub extension_id: SimExtensionId,
    pub message: String,
}

impl SimExtensionLoaderDiagnostic {
    fn new(
        code: impl Into<String>,
        extension_id: SimExtensionId,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            extension_id,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLoadReport {
    pub loaded: Vec<SimExtensionLoadedPack>,
    pub skipped: Vec<SimExtensionSkippedPack>,
    pub diagnostics: Vec<SimExtensionLoaderDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLoader {
    policy: SimExtensionPolicy,
}

impl SimExtensionLoader {
    pub fn new(policy: SimExtensionPolicy) -> Self {
        Self { policy }
    }

    pub fn load(
        &self,
        extensions: &[SimExtensionRecord],
        metadata: &BTreeMap<SimExtensionId, SimExtensionLoadMetadata>,
    ) -> SimExtensionLoadReport {
        let mut report = SimExtensionLoadReport::default();

        for extension in extensions {
            let metadata = metadata.get(&extension.id).cloned().unwrap_or_default();
            let policy_request = SimExtensionPolicyRequest::metadata_only()
                .with_script(metadata.prestartup_script.is_some())
                .with_install(!metadata.missing_dependencies.is_empty());
            let evaluation = self.policy.evaluate(extension, &policy_request);
            for diagnostic in evaluation.diagnostics {
                report.diagnostics.push(SimExtensionLoaderDiagnostic::new(
                    diagnostic.code,
                    diagnostic.extension_id,
                    diagnostic.message,
                ));
            }

            if !evaluation.decision.allows_loading() {
                report.skipped.push(SimExtensionSkippedPack {
                    extension_id: extension.id.clone(),
                    reason: "extension is not allowed to load by Sim policy".to_string(),
                });
                continue;
            }

            if metadata.prestartup_script.is_some() && !evaluation.permissions.script_allowed {
                report.skipped.push(SimExtensionSkippedPack {
                    extension_id: extension.id.clone(),
                    reason: "extension prestartup script is not permitted".to_string(),
                });
                continue;
            }

            if !metadata.missing_dependencies.is_empty() {
                for dependency in metadata.missing_dependencies {
                    report.diagnostics.push(SimExtensionLoaderDiagnostic::new(
                        SIM_EXTENSION_LOADER_MISSING_DEPENDENCY_CODE,
                        extension.id.clone(),
                        format!(
                            "extension dependency `{dependency}` is missing; install requires explicit approval"
                        ),
                    ));
                }
                report.skipped.push(SimExtensionSkippedPack {
                    extension_id: extension.id.clone(),
                    reason: "extension has missing dependencies".to_string(),
                });
                continue;
            }

            if let Some(import_error) = metadata.import_error {
                report.diagnostics.push(SimExtensionLoaderDiagnostic::new(
                    SIM_EXTENSION_LOADER_IMPORT_FAILED_CODE,
                    extension.id.clone(),
                    import_error,
                ));
                report.skipped.push(SimExtensionSkippedPack {
                    extension_id: extension.id.clone(),
                    reason: "extension import failed in native Sim loader".to_string(),
                });
                continue;
            }

            for hook in &metadata.protected_hook_changes {
                report.diagnostics.push(SimExtensionLoaderDiagnostic::new(
                    SIM_EXTENSION_LOADER_HOOK_RESTORED_CODE,
                    extension.id.clone(),
                    format!("protected Sim hook `{hook}` was restored after extension loading"),
                ));
            }

            report.loaded.push(SimExtensionLoadedPack {
                extension_id: extension.id.clone(),
                source_path: extension.source_path.clone(),
                policy_decision: evaluation.decision,
                prestartup_script_ran: metadata.prestartup_script.is_some(),
                restored_hooks: metadata.protected_hook_changes,
            });
        }

        report
    }
}
