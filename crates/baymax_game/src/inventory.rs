use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct BaymaxGameSourcePath(PathBuf);

impl BaymaxGameSourcePath {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MigrationDecision {
    NativeBaymaxFeature,
    BaymaxAdapter { owner: String },
    ExternalCommand { command: String },
    Excluded { reason: String },
}

impl MigrationDecision {
    pub fn is_excluded_without_reason(&self) -> bool {
        matches!(self, Self::Excluded { reason } if reason.trim().is_empty())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MigrationSpecCoverage {
    pub name: String,
    pub scope: String,
    pub location: PathBuf,
}

impl MigrationSpecCoverage {
    pub fn new(
        name: impl Into<String>,
        scope: impl Into<String>,
        location: impl Into<PathBuf>,
    ) -> Self {
        Self {
            name: name.into(),
            scope: scope.into(),
            location: location.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MigrationSourceArea {
    pub name: String,
    pub source: BaymaxGameSourcePath,
    pub scope: String,
    pub decision: MigrationDecision,
    pub spec_location: Option<PathBuf>,
}

impl MigrationSourceArea {
    pub fn new(
        name: impl Into<String>,
        source: impl Into<PathBuf>,
        scope: impl Into<String>,
        decision: MigrationDecision,
        spec_location: Option<impl Into<PathBuf>>,
    ) -> Self {
        Self {
            name: name.into(),
            source: BaymaxGameSourcePath::new(source),
            scope: scope.into(),
            decision,
            spec_location: spec_location.map(Into::into),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MigrationValidationError {
    MissingSpecScope {
        spec: String,
    },
    MissingSpecFile {
        spec: String,
        file: String,
    },
    MissingSourceAreaScope {
        source_area: String,
    },
    MissingSourceAreaSpecCoverage {
        source_area: String,
    },
    ExcludedSourceAreaWithoutBoundaryReason {
        source_area: String,
    },
    /// A task entry in a grouped spec's tasks.md is missing requirement references.
    MissingTaskRequirementRefs {
        task: String,
        spec: String,
    },
    /// A task entry in a grouped spec's tasks.md is missing expected write targets.
    MissingTaskWriteTargets {
        task: String,
        spec: String,
    },
    /// One of the ten expected Comfy harness spec directories is missing.
    MissingComfySpecDirectory {
        spec: String,
    },
    /// Two or more Comfy harness specs claim overlapping scope keywords.
    ComfyHarnessSpecOverlap {
        spec_a: String,
        spec_b: String,
        shared_keywords: String,
    },
    /// A world-model harness task uses Comfy-scope keywords without
    /// referencing an applicable Comfy spec or documenting a divergence.
    WorldModelHarnessTaskMissingComfyRef {
        task: String,
        spec: String,
        keywords: String,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MigrationValidationReport {
    pub errors: Vec<MigrationValidationError>,
}

impl MigrationValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn push(&mut self, error: MigrationValidationError) {
        self.errors.push(error);
    }
}

pub trait BaymaxGameMigrationInventory {
    fn validate_spec_pack(&self) -> MigrationValidationReport;
    fn classify_source_area(&self, path: &BaymaxGameSourcePath) -> MigrationDecision;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MigrationInventory {
    pub spec_root: PathBuf,
    pub grouped_specs: Vec<MigrationSpecCoverage>,
    pub source_areas: Vec<MigrationSourceArea>,
}

impl MigrationInventory {
    pub const REQUIRED_SPEC_FILES: [&'static str; 3] = ["requirements.md", "design.md", "tasks.md"];

    pub fn new(spec_root: impl Into<PathBuf>) -> Self {
        Self {
            spec_root: spec_root.into(),
            grouped_specs: Vec::new(),
            source_areas: Vec::new(),
        }
    }

    pub fn with_grouped_specs(mut self, grouped_specs: Vec<MigrationSpecCoverage>) -> Self {
        self.grouped_specs = grouped_specs;
        self
    }

    pub fn with_source_areas(mut self, source_areas: Vec<MigrationSourceArea>) -> Self {
        self.source_areas = source_areas;
        self
    }

    fn spec_path(&self, spec: &MigrationSpecCoverage) -> PathBuf {
        if spec.location.is_absolute() {
            spec.location.clone()
        } else {
            self.spec_root.join(&spec.location)
        }
    }

    fn spec_exists_for_location(&self, location: &Path) -> bool {
        self.grouped_specs
            .iter()
            .any(|spec| spec.location == location || self.spec_path(spec) == location)
    }
}

impl BaymaxGameMigrationInventory for MigrationInventory {
    fn validate_spec_pack(&self) -> MigrationValidationReport {
        let mut report = MigrationValidationReport::default();

        for spec in &self.grouped_specs {
            if spec.scope.trim().is_empty() {
                report.push(MigrationValidationError::MissingSpecScope {
                    spec: spec.name.clone(),
                });
            }

            let spec_path = self.spec_path(spec);
            for file in Self::REQUIRED_SPEC_FILES {
                if !spec_path.join(file).is_file() {
                    report.push(MigrationValidationError::MissingSpecFile {
                        spec: spec.name.clone(),
                        file: file.to_string(),
                    });
                }
            }
        }

        for source_area in &self.source_areas {
            if source_area.scope.trim().is_empty() {
                report.push(MigrationValidationError::MissingSourceAreaScope {
                    source_area: source_area.name.clone(),
                });
            }

            if source_area.decision.is_excluded_without_reason() {
                report.push(
                    MigrationValidationError::ExcludedSourceAreaWithoutBoundaryReason {
                        source_area: source_area.name.clone(),
                    },
                );
            }

            if !matches!(source_area.decision, MigrationDecision::Excluded { .. }) {
                let has_spec_coverage = source_area
                    .spec_location
                    .as_deref()
                    .is_some_and(|location| self.spec_exists_for_location(location));
                if !has_spec_coverage {
                    report.push(MigrationValidationError::MissingSourceAreaSpecCoverage {
                        source_area: source_area.name.clone(),
                    });
                }
            }
        }

        report
    }

    fn classify_source_area(&self, path: &BaymaxGameSourcePath) -> MigrationDecision {
        self.source_areas
            .iter()
            .filter(|source_area| path.as_path().starts_with(source_area.source.as_path()))
            .max_by_key(|source_area| source_area.source.as_path().components().count())
            .map(|source_area| source_area.decision.clone())
            .unwrap_or(MigrationDecision::Excluded {
                reason: "Source area is not listed in the Baymax game migration inventory"
                    .to_string(),
            })
    }
}
