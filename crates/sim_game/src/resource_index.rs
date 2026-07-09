use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    SimGameFormatClassification, SimGameFormatClassifier, SimGameFormatDiagnostic,
    SimGameFormatKind, SimGameResourceReference, SimGameTextResourceParser,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameResourceIndex {
    resources: Vec<SimGameIndexedResource>,
}

impl SimGameResourceIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_resource(
        mut self,
        path: impl AsRef<Path>,
        source: Option<&str>,
    ) -> SimGameResourceIndex {
        self.resources
            .push(SimGameIndexedResource::from_source(path, source));
        self
    }

    pub fn resources(&self) -> &[SimGameIndexedResource] {
        &self.resources
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = &SimGameFormatDiagnostic> {
        self.resources
            .iter()
            .flat_map(|resource| resource.diagnostics.iter())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameIndexedResource {
    pub path: PathBuf,
    pub classification: SimGameFormatClassification,
    pub references: Vec<SimGameResourceReference>,
    pub diagnostics: Vec<SimGameFormatDiagnostic>,
    pub parse_state: SimGameResourceParseState,
}

impl SimGameIndexedResource {
    pub fn from_source(path: impl AsRef<Path>, source: Option<&str>) -> Self {
        let path = path.as_ref();
        let classification = SimGameFormatClassifier::new().classify_path(path);
        let mut references = Vec::new();
        let mut diagnostics = Vec::new();
        let parse_state = match (classification.text_parse_supported, source) {
            (true, Some(source)) => {
                let parse = SimGameTextResourceParser::new().parse(source);
                references = parse.references;
                diagnostics = parse.diagnostics;
                if diagnostics.is_empty() {
                    SimGameResourceParseState::Complete
                } else {
                    SimGameResourceParseState::Partial
                }
            }
            (true, None) => {
                diagnostics.push(SimGameFormatDiagnostic {
                    code: "sim_game.resource.missing_source".to_string(),
                    message: "text resource metadata requires source content".to_string(),
                    line: None,
                });
                SimGameResourceParseState::Partial
            }
            (false, _) if classification.kind == SimGameFormatKind::Unknown => {
                if let Some(reason) = &classification.unsupported_reason {
                    diagnostics.push(SimGameFormatDiagnostic {
                        code: "sim_game.resource.unsupported_format".to_string(),
                        message: reason.clone(),
                        line: None,
                    });
                }
                SimGameResourceParseState::Unsupported
            }
            (false, _) => SimGameResourceParseState::MetadataOnly,
        };

        Self {
            path: path.to_path_buf(),
            classification,
            references,
            diagnostics,
            parse_state,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameResourceParseState {
    Complete,
    Partial,
    MetadataOnly,
    Unsupported,
}
