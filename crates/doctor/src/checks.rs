use std::path::{Path, PathBuf};

use crate::{DoctorCheck, DoctorCheckReport};

pub struct SystemDependencyCheck {
    name: String,
    executable: String,
}

pub struct ExtensionDirectoryCheck {
    path: PathBuf,
}

pub struct ProviderConnectivityCheck {
    name: String,
    check: Box<dyn Fn() -> anyhow::Result<()> + Send + Sync>,
}

impl SystemDependencyCheck {
    pub fn new(executable: impl Into<String>) -> Self {
        let executable = executable.into();
        Self {
            name: format!("system dependency: {executable}"),
            executable,
        }
    }
}

impl ExtensionDirectoryCheck {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl ProviderConnectivityCheck {
    pub fn new(
        name: impl Into<String>,
        check: impl Fn() -> anyhow::Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            check: Box::new(check),
        }
    }
}

impl DoctorCheck for SystemDependencyCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self) -> DoctorCheckReport {
        if executable_in_path(&self.executable) {
            DoctorCheckReport::pass(self.name(), format!("{} is available", self.executable))
        } else {
            DoctorCheckReport::fail(
                self.name(),
                format!("{} was not found on PATH", self.executable),
                format!("Install {} or add it to PATH", self.executable),
            )
        }
    }
}

impl DoctorCheck for ExtensionDirectoryCheck {
    fn name(&self) -> &str {
        "extensions directory"
    }

    fn run(&self) -> DoctorCheckReport {
        if self.path.is_dir() {
            DoctorCheckReport::pass(self.name(), format!("{} exists", self.path.display()))
        } else {
            DoctorCheckReport::warning(
                self.name(),
                format!("{} does not exist", self.path.display()),
                "Create the directory or update the configured extensions path",
            )
        }
    }
}

impl DoctorCheck for ProviderConnectivityCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self) -> DoctorCheckReport {
        match (self.check)() {
            Ok(()) => DoctorCheckReport::pass(self.name(), "provider connectivity check passed"),
            Err(error) => DoctorCheckReport::fail(
                self.name(),
                format!("provider connectivity check failed: {error}"),
                "Check provider credentials, network access, and configured endpoint",
            ),
        }
    }
}

fn executable_in_path(executable: &str) -> bool {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .any(|path| executable_exists(&path.join(executable)))
}

fn executable_exists(path: &Path) -> bool {
    if cfg!(windows) {
        path.is_file() || path.with_extension("exe").is_file()
    } else {
        path.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DoctorStatus;

    #[test]
    fn extension_directory_check_warns_for_missing_directory() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let report = ExtensionDirectoryCheck::new(temp_dir.path().join("missing")).run();

        assert_eq!(report.status, DoctorStatus::Warning);
        assert!(report.remediation.is_some());
    }

    #[test]
    fn provider_connectivity_check_reports_failure() {
        let report = ProviderConnectivityCheck::new("provider", || anyhow::bail!("no token")).run();

        assert_eq!(report.status, DoctorStatus::Fail);
        assert!(report.message.contains("no token"));
    }
}
