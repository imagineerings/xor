use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub variables: HashMap<String, String>,
    pub current_step: usize,
    pub step_results: Vec<StepResult>,
    pub secrets: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeOutput {
    pub success: bool,
    pub step_count: usize,
    pub completed_steps: usize,
    pub summary: String,
    pub step_results: Vec<StepResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub prompt: String,
    pub output: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}
