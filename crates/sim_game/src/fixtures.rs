use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Fixture source
// ---------------------------------------------------------------------------

/// The origin of a fixture — where it was copied or converted from.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum FixtureSource {
    /// A path within a Godot-compatible project.
    GodotProject {
        root: PathBuf,
        relative_path: PathBuf,
    },
    /// A path within a world-model project or workspace.
    WorldModelProject {
        root: PathBuf,
        relative_path: PathBuf,
    },
    /// An external URL the fixture was downloaded from.
    Url { url: String },
    /// A native Sim generated asset or node output.
    SimGeneratedAsset {
        generation_name: String,
        node_id: Option<String>,
    },
    /// Manually created fixture with no external source.
    Original,
}

impl FixtureSource {
    /// A human-readable description of the source.
    pub fn description(&self) -> String {
        match self {
            Self::GodotProject {
                root,
                relative_path,
            } => {
                format!(
                    "Godot project '{}': {}",
                    root.display(),
                    relative_path.display()
                )
            }
            Self::WorldModelProject {
                root,
                relative_path,
            } => {
                format!(
                    "World-model '{}': {}",
                    root.display(),
                    relative_path.display()
                )
            }
            Self::Url { url } => format!("URL: {url}"),
            Self::SimGeneratedAsset {
                generation_name,
                node_id,
            } => {
                if let Some(nid) = node_id {
                    format!("Sim generated asset '{generation_name}' node {nid}")
                } else {
                    format!("Sim generated asset '{generation_name}'")
                }
            }
            Self::Original => "Original fixture, no external source".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Fixture license
// ---------------------------------------------------------------------------

/// License or attribution terms for a fixture.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum FixtureLicense {
    /// SPDX license identifier (e.g., "MIT", "CC0-1.0", "Apache-2.0").
    Spdx(String),
    /// Custom attribution text.
    Custom(String),
    /// Unlicensed — all rights reserved by the original author.
    Unlicensed { author: Option<String> },
}

impl FixtureLicense {
    /// Returns a short label for the license.
    pub fn label(&self) -> String {
        match self {
            Self::Spdx(id) => id.clone(),
            Self::Custom(text) => format!("Custom: {text}"),
            Self::Unlicensed { author: Some(a) } => format!("Unlicensed © {a}"),
            Self::Unlicensed { author: None } => "Unlicensed".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Fixture attribution
// ---------------------------------------------------------------------------

/// Metadata record for a fixture that was copied, converted, or created,
/// preserving source attribution.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct FixtureAttribution {
    /// Relative path of the fixture within the Sim workspace.
    pub fixture_path: PathBuf,
    /// Where the fixture originated.
    pub source: FixtureSource,
    /// License or attribution terms.
    pub license: FixtureLicense,
    /// Optional original author or upstream project name.
    pub author: Option<String>,
    /// Optional notes about conversion or adaptation.
    pub notes: Option<String>,
}

impl FixtureAttribution {
    pub fn new(
        fixture_path: impl Into<PathBuf>,
        source: FixtureSource,
        license: FixtureLicense,
    ) -> Self {
        Self {
            fixture_path: fixture_path.into(),
            source,
            license,
            author: None,
            notes: None,
        }
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Fixture manifest
// ---------------------------------------------------------------------------

/// A collection of fixture attributions for a project or migration spec.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FixtureManifest {
    pub fixtures: Vec<FixtureAttribution>,
}

impl FixtureManifest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, attr: FixtureAttribution) {
        self.fixtures.push(attr);
    }

    pub fn extend(&mut self, other: Self) {
        self.fixtures.extend(other.fixtures);
    }

    pub fn is_empty(&self) -> bool {
        self.fixtures.is_empty()
    }

    pub fn len(&self) -> usize {
        self.fixtures.len()
    }

    pub fn find_by_path(&self, path: &Path) -> Vec<&FixtureAttribution> {
        self.fixtures
            .iter()
            .filter(|f| f.fixture_path == path)
            .collect()
    }

    pub fn validate(&self) -> FixtureAttributionReport {
        FixtureAttributionValidator::new().validate(self)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FixtureAttributionValidator;

impl FixtureAttributionValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, manifest: &FixtureManifest) -> FixtureAttributionReport {
        let mut diagnostics = Vec::new();

        for (index, fixture) in manifest.fixtures.iter().enumerate() {
            if fixture.fixture_path.as_os_str().is_empty() {
                diagnostics.push(FixtureAttributionDiagnostic {
                    index,
                    field: "fixture_path",
                    message: "fixture path is required".to_string(),
                });
            }

            match &fixture.source {
                FixtureSource::Url { url } if url.trim().is_empty() => {
                    diagnostics.push(FixtureAttributionDiagnostic {
                        index,
                        field: "source.url",
                        message: "fixture URL source is required".to_string(),
                    });
                }
                FixtureSource::GodotProject {
                    root,
                    relative_path,
                }
                | FixtureSource::WorldModelProject {
                    root,
                    relative_path,
                } => {
                    if root.as_os_str().is_empty() {
                        diagnostics.push(FixtureAttributionDiagnostic {
                            index,
                            field: "source.root",
                            message: "fixture source root is required".to_string(),
                        });
                    }
                    if relative_path.as_os_str().is_empty() {
                        diagnostics.push(FixtureAttributionDiagnostic {
                            index,
                            field: "source.relative_path",
                            message: "fixture source relative path is required".to_string(),
                        });
                    }
                }
                FixtureSource::SimGeneratedAsset {
                    generation_name, ..
                } if generation_name.trim().is_empty() => {
                    diagnostics.push(FixtureAttributionDiagnostic {
                        index,
                        field: "source.generation_name",
                        message: "Sim generated asset source name is required".to_string(),
                    });
                }
                FixtureSource::SimGeneratedAsset { .. } | FixtureSource::Url { .. } => {}
                FixtureSource::Original => {}
            }

            match &fixture.license {
                FixtureLicense::Spdx(identifier) if identifier.trim().is_empty() => {
                    diagnostics.push(FixtureAttributionDiagnostic {
                        index,
                        field: "license.spdx",
                        message: "SPDX license identifier is required".to_string(),
                    });
                }
                FixtureLicense::Custom(text) if text.trim().is_empty() => {
                    diagnostics.push(FixtureAttributionDiagnostic {
                        index,
                        field: "license.custom",
                        message: "custom license text is required".to_string(),
                    });
                }
                FixtureLicense::Unlicensed { author } => {
                    if author
                        .as_deref()
                        .is_none_or(|author| author.trim().is_empty())
                    {
                        diagnostics.push(FixtureAttributionDiagnostic {
                            index,
                            field: "license.author",
                            message: "unlicensed fixtures require an author".to_string(),
                        });
                    }
                }
                FixtureLicense::Spdx(_) | FixtureLicense::Custom(_) => {}
            }
        }

        FixtureAttributionReport { diagnostics }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureAttributionReport {
    pub diagnostics: Vec<FixtureAttributionDiagnostic>,
}

impl FixtureAttributionReport {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureAttributionDiagnostic {
    pub index: usize,
    pub field: &'static str,
    pub message: String,
}

impl IntoIterator for FixtureManifest {
    type Item = FixtureAttribution;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fixtures.into_iter()
    }
}

impl FromIterator<FixtureAttribution> for FixtureManifest {
    fn from_iter<I: IntoIterator<Item = FixtureAttribution>>(iter: I) -> Self {
        Self {
            fixtures: iter.into_iter().collect(),
        }
    }
}
