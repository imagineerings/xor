use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameExecutableSettings {
    pub executable_path: Option<PathBuf>,
    pub diagnostics: Vec<SimGameExecutableDiagnostic>,
}

impl SimGameExecutableSettings {
    pub fn configured(path: impl Into<PathBuf>) -> Self {
        Self {
            executable_path: Some(path.into()),
            diagnostics: Vec::new(),
        }
    }

    pub fn missing() -> Self {
        Self {
            executable_path: None,
            diagnostics: vec![SimGameExecutableDiagnostic {
                code: "sim_game.executable.missing".to_string(),
                message: "configure a game engine executable before creating run/export tasks"
                    .to_string(),
            }],
        }
    }

    pub fn is_configured(&self) -> bool {
        self.executable_path.is_some() && self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameExecutableDiagnostic {
    pub code: String,
    pub message: String,
}
