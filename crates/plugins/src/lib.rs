use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PLUGIN_MANIFEST_NAMES: &[&str] = &[
    "sim-plugin.json",
    "plugin.json",
    ".codex-plugin/plugin.json",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plugin {
    pub root_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: PluginManifest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub format: PluginFormat,
    #[serde(default)]
    pub entrypoint: Option<PathBuf>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginFormat {
    #[default]
    Manifest,
    Command,
    Wasm,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginDiscovery {
    pub plugins: Vec<Plugin>,
    pub errors: Vec<PluginLoadError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginLoadError {
    pub path: PathBuf,
    pub kind: PluginLoadErrorKind,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PluginLoadErrorKind {
    #[error("failed to read directory: {0}")]
    ReadDirectory(String),
    #[error("failed to read manifest: {0}")]
    ReadManifest(String),
    #[error("failed to parse manifest: {0}")]
    ParseManifest(String),
    #[error("manifest is missing required entrypoint for {format:?} plugin")]
    MissingEntrypoint { format: PluginFormat },
    #[error("entrypoint path escapes plugin root")]
    EntrypointEscapesRoot,
}

pub fn discover_plugins(
    directories: impl IntoIterator<Item = impl AsRef<Path>>,
) -> PluginDiscovery {
    let mut discovery = PluginDiscovery::default();

    for directory in directories {
        discover_plugins_in_directory(directory.as_ref(), &mut discovery);
    }

    discovery
}

pub fn load_plugin(root_dir: impl AsRef<Path>) -> Result<Plugin> {
    let root_dir = root_dir.as_ref();
    let manifest_path = find_manifest(root_dir)
        .with_context(|| format!("no plugin manifest found in {}", root_dir.display()))?;
    load_plugin_manifest(root_dir, &manifest_path)
}

fn discover_plugins_in_directory(directory: &Path, discovery: &mut PluginDiscovery) {
    if let Ok(plugin) = load_plugin(directory) {
        discovery.plugins.push(plugin);
        return;
    }

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            discovery.errors.push(PluginLoadError {
                path: directory.to_path_buf(),
                kind: PluginLoadErrorKind::ReadDirectory(error.to_string()),
            });
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                discovery.errors.push(PluginLoadError {
                    path: directory.to_path_buf(),
                    kind: PluginLoadErrorKind::ReadDirectory(error.to_string()),
                });
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        match load_plugin(&path) {
            Ok(plugin) => discovery.plugins.push(plugin),
            Err(error) => {
                if find_manifest(&path).is_some() {
                    discovery.errors.push(PluginLoadError {
                        path,
                        kind: PluginLoadErrorKind::ParseManifest(error.to_string()),
                    });
                }
            }
        }
    }
}

fn find_manifest(root_dir: &Path) -> Option<PathBuf> {
    PLUGIN_MANIFEST_NAMES
        .iter()
        .map(|name| root_dir.join(name))
        .find(|path| path.is_file())
}

fn load_plugin_manifest(root_dir: &Path, manifest_path: &Path) -> Result<Plugin> {
    let manifest_text = fs::read_to_string(manifest_path).map_err(|error| PluginLoadError {
        path: manifest_path.to_path_buf(),
        kind: PluginLoadErrorKind::ReadManifest(error.to_string()),
    })?;
    let manifest = serde_json::from_str::<PluginManifest>(&manifest_text).map_err(|error| {
        PluginLoadError {
            path: manifest_path.to_path_buf(),
            kind: PluginLoadErrorKind::ParseManifest(error.to_string()),
        }
    })?;

    validate_manifest(root_dir, &manifest)?;

    Ok(Plugin {
        root_dir: root_dir.to_path_buf(),
        manifest_path: manifest_path.to_path_buf(),
        manifest,
    })
}

fn validate_manifest(root_dir: &Path, manifest: &PluginManifest) -> Result<()> {
    match manifest.format {
        PluginFormat::Manifest => {}
        PluginFormat::Command | PluginFormat::Wasm => {
            let entrypoint = manifest
                .entrypoint
                .as_ref()
                .ok_or_else(|| PluginLoadError {
                    path: root_dir.to_path_buf(),
                    kind: PluginLoadErrorKind::MissingEntrypoint {
                        format: manifest.format,
                    },
                })?;

            if entrypoint.is_absolute()
                || entrypoint
                    .components()
                    .any(|component| matches!(component, Component::ParentDir))
            {
                return Err(PluginLoadError {
                    path: root_dir.join(entrypoint),
                    kind: PluginLoadErrorKind::EntrypointEscapesRoot,
                }
                .into());
            }
        }
    }

    Ok(())
}

impl std::fmt::Display for PluginLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.kind)
    }
}

impl std::error::Error for PluginLoadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_plugin_manifest() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let plugin_dir = temp_dir.path().join("example");
        fs::create_dir(&plugin_dir).expect("create plugin dir");
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "id": "example",
                "name": "Example",
                "version": "1.0.0",
                "format": "command",
                "entrypoint": "run.sh",
                "capabilities": ["tools"]
            }"#,
        )
        .expect("write manifest");

        let plugin = load_plugin(&plugin_dir).expect("load plugin");

        assert_eq!(plugin.manifest.id, "example");
        assert_eq!(plugin.manifest.format, PluginFormat::Command);
        assert_eq!(plugin.manifest.entrypoint, Some(PathBuf::from("run.sh")));
    }

    #[test]
    fn discovers_plugins_and_reports_bad_manifests() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let good_dir = temp_dir.path().join("good");
        let bad_dir = temp_dir.path().join("bad");
        fs::create_dir(&good_dir).expect("create good plugin dir");
        fs::create_dir(&bad_dir).expect("create bad plugin dir");
        fs::write(
            good_dir.join("sim-plugin.json"),
            r#"{ "id": "good", "name": "Good", "version": "1.0.0" }"#,
        )
        .expect("write good manifest");
        fs::write(bad_dir.join("plugin.json"), r#"{ "id": "#).expect("write bad manifest");

        let discovery = discover_plugins([temp_dir.path()]);

        assert_eq!(discovery.plugins.len(), 1);
        assert_eq!(discovery.plugins[0].manifest.id, "good");
        assert_eq!(discovery.errors.len(), 1);
    }

    #[test]
    fn rejects_entrypoints_that_escape_root() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(
            temp_dir.path().join("plugin.json"),
            r#"{
                "id": "escape",
                "name": "Escape",
                "version": "1.0.0",
                "format": "command",
                "entrypoint": "../run.sh"
            }"#,
        )
        .expect("write manifest");

        let error = load_plugin(temp_dir.path()).expect_err("plugin should fail");

        assert!(error.to_string().contains("entrypoint path escapes"));
    }
}
