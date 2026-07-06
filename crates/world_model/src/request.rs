use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// World model profile
// ---------------------------------------------------------------------------

/// A named profile describing a model configuration for generation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WorldModelProfile {
    pub name: String,
    pub family: String,
    pub variant: Option<String>,
}

impl WorldModelProfile {
    pub fn new(name: impl Into<String>, family: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            family: family.into(),
            variant: None,
        }
    }

    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Action controls
// ---------------------------------------------------------------------------

/// A single action control input (e.g., WASD or IJKL key state).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldActionControl {
    pub name: String,
    pub value: f32,
    pub frame: u64,
}

impl WorldActionControl {
    pub fn new(name: impl Into<String>, value: f32, frame: u64) -> Self {
        Self {
            name: name.into(),
            value,
            frame,
        }
    }
}

/// A set of action controls captured at a point in time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldControl {
    pub actions: Vec<WorldActionControl>,
    pub frame_count: u64,
}

impl WorldControl {
    pub fn new(actions: Vec<WorldActionControl>, frame_count: u64) -> Self {
        Self {
            actions,
            frame_count,
        }
    }

    /// Validate WASD/IJKL semantics per Requirement 5.2.
    ///
    /// Returns a list of validation errors. An empty vec means the control
    /// set is valid.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Check that no mutually exclusive direction keys are pressed
        // simultaneously with non-zero values.
        let wasd_horiz: Vec<&WorldActionControl> = self
            .actions
            .iter()
            .filter(|a| a.name == "w" || a.name == "s")
            .collect();
        let wasd_vert: Vec<&WorldActionControl> = self
            .actions
            .iter()
            .filter(|a| a.name == "a" || a.name == "d")
            .collect();

        if wasd_horiz.iter().filter(|a| a.value != 0.0).count() > 1 {
            errors.push("W and S cannot both be pressed at the same time".to_string());
        }
        if wasd_vert.iter().filter(|a| a.value != 0.0).count() > 1 {
            errors.push("A and D cannot both be pressed at the same time".to_string());
        }

        // Reject NaN values.
        for action in &self.actions {
            if action.value.is_nan() {
                errors.push(format!("Action '{}' has NaN value", action.name));
            }
        }

        errors
    }
}

// ---------------------------------------------------------------------------
// Generation request
// ---------------------------------------------------------------------------

/// A request to generate world content through a model backend.
///
/// Captures prompt, source image, controls, model profile, seed, and output
/// target (Requirement 5.1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldGenerationRequest {
    pub prompt: String,
    pub source_image: Option<String>,
    pub controls: Vec<WorldControl>,
    pub model_profile: WorldModelProfile,
    pub seed: Option<u64>,
    pub output_target: String,
}

impl WorldGenerationRequest {
    pub fn new(
        prompt: impl Into<String>,
        model_profile: WorldModelProfile,
        output_target: impl Into<String>,
    ) -> Self {
        Self {
            prompt: prompt.into(),
            source_image: None,
            controls: Vec::new(),
            model_profile,
            seed: None,
            output_target: output_target.into(),
        }
    }

    pub fn with_source_image(mut self, path: impl Into<String>) -> Self {
        self.source_image = Some(path.into());
        self
    }

    pub fn with_controls(mut self, controls: Vec<WorldControl>) -> Self {
        self.controls = controls;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}
