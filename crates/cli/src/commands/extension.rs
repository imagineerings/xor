use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "extension", about = "Manage local Sim extensions")]
struct ExtensionArgs {
    #[arg(long, value_name = "FILE", hide = true)]
    registry: Option<PathBuf>,
    #[command(subcommand)]
    command: ExtensionCommand,
}

#[derive(Subcommand, Debug)]
enum ExtensionCommand {
    /// List installed extensions with status.
    List,
    /// Add an extension path to the local registry.
    Add { path: PathBuf },
    /// Remove an extension by name.
    Remove { name: String },
    /// Show extension connection status.
    Status { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledExtension {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
    pub added_at_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionStatus {
    Ready,
    MissingPath,
    MissingManifest,
    Disabled,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionRegistry {
    extensions: Vec<InstalledExtension>,
}

impl ExtensionRegistry {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("reading extension registry {}", path.display()))?;
        if content.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(&content)
            .with_context(|| format!("parsing extension registry {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("creating extension registry directory {}", parent.display())
            })?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, format!("{content}\n"))
            .with_context(|| format!("writing extension registry {}", path.display()))
    }

    pub fn extensions(&self) -> &[InstalledExtension] {
        &self.extensions
    }

    pub fn add_path(&mut self, path: PathBuf) -> Result<&InstalledExtension> {
        if !path.exists() {
            return Err(anyhow!("extension path does not exist: {}", path.display()));
        }
        let name = extension_name_from_path(&path)?;
        if self
            .extensions
            .iter()
            .any(|extension| extension.name == name)
        {
            return Err(anyhow!("extension `{name}` is already installed"));
        }
        self.extensions.push(InstalledExtension {
            name,
            path,
            enabled: true,
            added_at_unix_seconds: now_unix_seconds()?,
        });
        self.extensions
            .last()
            .ok_or_else(|| anyhow!("failed to add extension"))
    }

    pub fn remove(&mut self, name: &str) -> Result<InstalledExtension> {
        let Some(index) = self
            .extensions
            .iter()
            .position(|extension| extension.name == name)
        else {
            return Err(anyhow!("extension `{name}` is not installed"));
        };
        Ok(self.extensions.remove(index))
    }

    pub fn find(&self, name: &str) -> Option<&InstalledExtension> {
        self.extensions
            .iter()
            .find(|extension| extension.name == name)
    }
}

impl InstalledExtension {
    pub fn status(&self) -> ExtensionStatus {
        if !self.enabled {
            return ExtensionStatus::Disabled;
        }
        if !self.path.exists() {
            return ExtensionStatus::MissingPath;
        }
        if !extension_manifest_exists(&self.path) {
            return ExtensionStatus::MissingManifest;
        }
        ExtensionStatus::Ready
    }
}

impl ExtensionStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::MissingPath => "missing-path",
            Self::MissingManifest => "missing-manifest",
            Self::Disabled => "disabled",
        }
    }
}

pub fn registry_path() -> PathBuf {
    paths::config_dir().join("extensions.json")
}

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let args = ExtensionArgs::try_parse_from(args)?;
    let path = args.registry.unwrap_or_else(registry_path);
    let mut registry = ExtensionRegistry::load(&path)?;
    let mut output = io::stdout();

    match args.command {
        ExtensionCommand::List => cmd_list(&registry, &mut output),
        ExtensionCommand::Add {
            path: extension_path,
        } => {
            let extension = registry.add_path(extension_path)?;
            writeln!(output, "Added extension `{}`", extension.name)?;
            registry.save(&path)
        }
        ExtensionCommand::Remove { name } => {
            let extension = registry.remove(&name)?;
            writeln!(output, "Removed extension `{}`", extension.name)?;
            registry.save(&path)
        }
        ExtensionCommand::Status { name } => cmd_status(&registry, &name, &mut output),
    }
}

fn cmd_list(registry: &ExtensionRegistry, output: &mut impl Write) -> Result<()> {
    if registry.extensions().is_empty() {
        writeln!(output, "No extensions installed")?;
        return Ok(());
    }

    for extension in registry.extensions() {
        writeln!(
            output,
            "{}\t{}\t{}",
            extension.name,
            extension.status().label(),
            extension.path.display()
        )?;
    }
    Ok(())
}

fn cmd_status(registry: &ExtensionRegistry, name: &str, output: &mut impl Write) -> Result<()> {
    let extension = registry
        .find(name)
        .ok_or_else(|| anyhow!("extension `{name}` is not installed"))?;
    writeln!(output, "{}: {}", extension.name, extension.status().label())?;
    writeln!(output, "path: {}", extension.path.display())?;
    Ok(())
}

fn extension_name_from_path(path: &Path) -> Result<String> {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| anyhow!("could not derive extension name from {}", path.display()))
}

fn extension_manifest_exists(path: &Path) -> bool {
    if path.is_dir() {
        return path.join("extension.toml").exists() || path.join("manifest.json").exists();
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "extension.toml" || name == "manifest.json")
}

fn now_unix_seconds() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_lists_and_removes_extensions() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let extension_dir = temp_dir.path().join("demo-extension");
        fs::create_dir(&extension_dir)?;
        fs::write(extension_dir.join("extension.toml"), "id = \"demo\"")?;
        let registry_path = temp_dir.path().join("extensions.json");
        let mut registry = ExtensionRegistry::default();

        let extension = registry.add_path(extension_dir.clone())?;
        assert_eq!(extension.name, "demo-extension");
        assert_eq!(extension.status(), ExtensionStatus::Ready);
        registry.save(&registry_path)?;

        let mut loaded = ExtensionRegistry::load(&registry_path)?;
        assert_eq!(loaded.extensions().len(), 1);
        let removed = loaded.remove("demo-extension")?;
        assert_eq!(removed.path, extension_dir);
        assert!(loaded.extensions().is_empty());
        Ok(())
    }

    #[test]
    fn reports_missing_manifest_status() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let extension_dir = temp_dir.path().join("no-manifest");
        fs::create_dir(&extension_dir)?;
        let mut registry = ExtensionRegistry::default();

        let extension = registry.add_path(extension_dir)?;

        assert_eq!(extension.status(), ExtensionStatus::MissingManifest);
        Ok(())
    }

    #[test]
    fn prints_list_and_status() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let extension_dir = temp_dir.path().join("demo");
        fs::create_dir(&extension_dir)?;
        fs::write(extension_dir.join("manifest.json"), "{}")?;
        let mut registry = ExtensionRegistry::default();
        registry.add_path(extension_dir)?;

        let mut list_output = Vec::new();
        cmd_list(&registry, &mut list_output)?;
        let list_output = String::from_utf8(list_output)?;
        assert!(list_output.contains("demo"));
        assert!(list_output.contains("ready"));

        let mut status_output = Vec::new();
        cmd_status(&registry, "demo", &mut status_output)?;
        assert!(String::from_utf8(status_output)?.contains("demo: ready"));
        Ok(())
    }

    #[test]
    fn rejects_duplicate_extensions() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let extension_dir = temp_dir.path().join("demo");
        fs::create_dir(&extension_dir)?;
        let mut registry = ExtensionRegistry::default();

        registry.add_path(extension_dir.clone())?;
        let error = registry
            .add_path(extension_dir)
            .expect_err("duplicate should fail");

        assert!(error.to_string().contains("already installed"));
        Ok(())
    }
}
