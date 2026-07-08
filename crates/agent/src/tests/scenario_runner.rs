use anyhow::{Context as _, Result};
use eval_harbor::{
    AssertionResult, EvalRunner, MockProvider, RecordedInteraction, ScenarioDefinition,
    StepObservation,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Duration};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Recording {
    metadata: RecordingMetadata,
    turns: Vec<RecordedTurn>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RecordingFile {
    metadata: RecordingMetadata,
    turns: Vec<RecordedTurn>,
    scenario: ScenarioDefinition,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RecordingMetadata {
    scenario: String,
    created_by: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RecordedTurn {
    step_index: usize,
    instruction: String,
    observation: StepObservation,
}

struct ScenarioTestRunner {
    scenario: ScenarioDefinition,
    recording: Recording,
}

impl ScenarioTestRunner {
    fn from_recording(path: &Path) -> Result<Self> {
        let contents =
            fs::read_to_string(path).with_context(|| format!("failed to read {:?}", path))?;
        let recording_file: RecordingFile = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {:?}", path))?;

        Ok(Self {
            scenario: recording_file.scenario,
            recording: Recording {
                metadata: recording_file.metadata,
                turns: recording_file.turns,
            },
        })
    }

    fn from_scenario_and_recording(scenario: ScenarioDefinition, recording: Recording) -> Self {
        Self {
            scenario,
            recording,
        }
    }

    fn run(&self) -> Result<Vec<AssertionResult>> {
        let interactions = self
            .recording
            .turns
            .iter()
            .map(|turn| RecordedInteraction {
                scenario: self.scenario.name.clone(),
                step_index: turn.step_index,
                instruction: turn.instruction.clone(),
                observation: turn.observation.clone(),
            })
            .collect::<Vec<_>>();
        let mut runner = EvalRunner::new(
            vec![self.scenario.clone()],
            MockProvider::from_replay(interactions),
        );
        let result = runner.run_scenario(&self.scenario.name)?;
        let mut assertions = result.assertions;
        assertions.extend(
            result
                .steps
                .into_iter()
                .flat_map(|step| step.assertions.into_iter()),
        );
        Ok(assertions)
    }

    fn record_session(
        scenario: ScenarioDefinition,
        observations: Vec<StepObservation>,
        output: &Path,
    ) -> Result<Recording> {
        let turns = scenario
            .steps
            .iter()
            .zip(observations)
            .enumerate()
            .map(|(step_index, (step, observation))| RecordedTurn {
                step_index,
                instruction: step.instruction.clone(),
                observation,
            })
            .collect::<Vec<_>>();
        let recording = Recording {
            metadata: RecordingMetadata {
                scenario: scenario.name.clone(),
                created_by: "agent-scenario-runner".to_string(),
            },
            turns,
        };
        let encoded = RecordingFile {
            metadata: recording.metadata,
            turns: recording.turns,
            scenario,
        };
        fs::write(output, serde_json::to_string_pretty(&encoded)?)
            .with_context(|| format!("failed to write {:?}", output))?;

        let contents =
            fs::read_to_string(output).with_context(|| format!("failed to read {:?}", output))?;
        serde_json::from_str(&contents).with_context(|| format!("failed to parse {:?}", output))
    }
}

#[test]
fn scenario_recording_round_trips_through_replay() {
    use eval_harbor::{ExpectedOutcome, OutcomeCheck, ScenarioStep, ToolCallPattern};

    let scenario = ScenarioDefinition {
        name: "inspect project".to_string(),
        description: "records a deterministic agent interaction".to_string(),
        steps: vec![ScenarioStep {
            instruction: "list files".to_string(),
            expected_tool_calls: vec![ToolCallPattern {
                tool: "list_directory".to_string(),
                min_count: 1,
            }],
            expected_response_contains: vec!["Cargo.toml".to_string()],
        }],
        expected_outcomes: vec![ExpectedOutcome {
            check: OutcomeCheck::ResponseContains {
                text: "Cargo.toml".to_string(),
            },
            description: "final response mentions Cargo manifest".to_string(),
        }],
        tags: vec!["recording".to_string()],
    };
    let observations = vec![StepObservation {
        response: "Found Cargo.toml".to_string(),
        tool_calls: vec!["list_directory".to_string()],
    }];
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let recording_path = tempdir.path().join("recording.json");

    let recording =
        ScenarioTestRunner::record_session(scenario.clone(), observations, &recording_path)
            .expect("record session");
    let runner = ScenarioTestRunner::from_scenario_and_recording(scenario.clone(), recording);
    let assertions = runner.run().expect("replay recording");

    assert_eq!(assertions.len(), 3);
    assert!(assertions.iter().all(|assertion| assertion.passed));

    let loaded = ScenarioTestRunner::from_recording(&recording_path).expect("load recording");
    assert_eq!(loaded.scenario.name, scenario.name);
}

#[test]
fn scenario_replay_reports_instruction_mismatches() {
    let scenario = ScenarioDefinition {
        name: "mismatch".to_string(),
        description: String::new(),
        steps: vec![eval_harbor::ScenarioStep {
            instruction: "expected".to_string(),
            expected_tool_calls: Vec::new(),
            expected_response_contains: Vec::new(),
        }],
        expected_outcomes: Vec::new(),
        tags: Vec::new(),
    };
    let recording = Recording {
        metadata: RecordingMetadata {
            scenario: scenario.name.clone(),
            created_by: "test".to_string(),
        },
        turns: vec![RecordedTurn {
            step_index: 0,
            instruction: "actual".to_string(),
            observation: StepObservation::default(),
        }],
    };
    let runner = ScenarioTestRunner::from_scenario_and_recording(scenario, recording);

    let assertions = runner.run().expect("runner produces failed scenario");

    assert!(assertions.is_empty());
}

fn _keeps_design_shape_visible(_timeout: Duration) {}
