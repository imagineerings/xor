pub mod assertion;
pub mod gym;
pub mod mock_provider;
pub mod runner;

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub use assertion::AssertionEngine;
pub use gym::{
    ComparisonSummary, EvalTask, ModelComparison, ModelEvalConfig, ModelGym, ModelResult,
    TaskResult,
};
pub use mock_provider::{MockProvider, MockResponse, RecordedInteraction};
pub use runner::{EvalRunner, ScenarioExecutor};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScenarioDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub steps: Vec<ScenarioStep>,
    #[serde(default)]
    pub expected_outcomes: Vec<ExpectedOutcome>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScenarioStep {
    pub instruction: String,
    #[serde(default)]
    pub expected_tool_calls: Vec<ToolCallPattern>,
    #[serde(default)]
    pub expected_response_contains: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolCallPattern {
    pub tool: String,
    #[serde(default = "default_min_count")]
    pub min_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpectedOutcome {
    pub check: OutcomeCheck,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutcomeCheck {
    ToolCalled { tool: String, min_count: usize },
    ResponseContains { text: String },
    ResponseMatches { pattern: String },
    FinalOutput { validator: String },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct StepObservation {
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub tool_calls: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StepResult {
    pub instruction: String,
    pub observation: StepObservation,
    pub assertions: Vec<AssertionResult>,
    pub passed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AssertionResult {
    pub assertion: String,
    pub passed: bool,
    pub actual: String,
    pub expected: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScenarioResult {
    pub name: String,
    pub passed: bool,
    pub steps: Vec<StepResult>,
    pub assertions: Vec<AssertionResult>,
    pub duration_millis: u128,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvalSuiteResult {
    pub total_scenarios: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub scenarios: Vec<ScenarioResult>,
    pub duration_millis: u128,
}

impl EvalSuiteResult {
    pub fn to_json_string(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn duration(&self) -> Duration {
        duration_from_millis(self.duration_millis)
    }
}

impl ScenarioResult {
    pub fn duration(&self) -> Duration {
        duration_from_millis(self.duration_millis)
    }
}

fn duration_from_millis(millis: u128) -> Duration {
    let millis = u64::try_from(millis).unwrap_or(u64::MAX);
    Duration::from_millis(millis)
}

fn default_min_count() -> usize {
    1
}
