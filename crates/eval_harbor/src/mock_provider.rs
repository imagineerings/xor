use crate::{ScenarioDefinition, ScenarioExecutor, StepObservation};
use anyhow::{Context as _, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MockResponse {
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub tool_calls: Vec<String>,
}

impl From<MockResponse> for StepObservation {
    fn from(value: MockResponse) -> Self {
        StepObservation {
            response: value.response,
            tool_calls: value.tool_calls,
        }
    }
}

impl From<StepObservation> for MockResponse {
    fn from(value: StepObservation) -> Self {
        MockResponse {
            response: value.response,
            tool_calls: value.tool_calls,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecordedInteraction {
    pub scenario: String,
    pub step_index: usize,
    pub instruction: String,
    pub observation: StepObservation,
}

#[derive(Clone, Debug, Default)]
pub struct MockProvider {
    canned_responses: VecDeque<MockResponse>,
    replay_interactions: VecDeque<RecordedInteraction>,
    recorded_interactions: Vec<RecordedInteraction>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_responses(responses: impl IntoIterator<Item = MockResponse>) -> Self {
        Self {
            canned_responses: responses.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn from_replay(interactions: impl IntoIterator<Item = RecordedInteraction>) -> Self {
        Self {
            replay_interactions: interactions.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn push_response(&mut self, response: MockResponse) {
        self.canned_responses.push_back(response);
    }

    pub fn recorded_interactions(&self) -> &[RecordedInteraction] {
        &self.recorded_interactions
    }

    pub fn into_recorded_interactions(self) -> Vec<RecordedInteraction> {
        self.recorded_interactions
    }

    fn next_observation(
        &mut self,
        scenario: &ScenarioDefinition,
        step_index: usize,
    ) -> Result<StepObservation> {
        let step = scenario
            .steps
            .get(step_index)
            .with_context(|| format!("scenario {:?} has no step {step_index}", scenario.name))?;

        let observation = if let Some(interaction) = self.replay_interactions.pop_front() {
            if interaction.scenario != scenario.name
                || interaction.step_index != step_index
                || interaction.instruction != step.instruction
            {
                return Err(anyhow!(
                    "replay interaction mismatch: expected {:?} step {} instruction {:?}, got {:?} step {} instruction {:?}",
                    scenario.name,
                    step_index,
                    step.instruction,
                    interaction.scenario,
                    interaction.step_index,
                    interaction.instruction
                ));
            }
            interaction.observation
        } else {
            self.canned_responses
                .pop_front()
                .with_context(|| {
                    format!(
                        "mock provider has no canned response for {:?} step {step_index}",
                        scenario.name
                    )
                })?
                .into()
        };

        self.recorded_interactions.push(RecordedInteraction {
            scenario: scenario.name.clone(),
            step_index,
            instruction: step.instruction.clone(),
            observation: observation.clone(),
        });

        Ok(observation)
    }
}

impl ScenarioExecutor for MockProvider {
    fn execute_step(
        &mut self,
        scenario: &ScenarioDefinition,
        step_index: usize,
    ) -> Result<StepObservation> {
        self.next_observation(scenario, step_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvalRunner, ScenarioStep};

    fn scenario() -> ScenarioDefinition {
        ScenarioDefinition {
            name: "deterministic".to_string(),
            description: String::new(),
            steps: vec![
                ScenarioStep {
                    instruction: "first".to_string(),
                    expected_tool_calls: Vec::new(),
                    expected_response_contains: vec!["one".to_string()],
                },
                ScenarioStep {
                    instruction: "second".to_string(),
                    expected_tool_calls: Vec::new(),
                    expected_response_contains: vec!["two".to_string()],
                },
            ],
            expected_outcomes: Vec::new(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn canned_responses_are_consumed_deterministically() {
        let provider = MockProvider::from_responses([
            MockResponse {
                response: "one".to_string(),
                tool_calls: Vec::new(),
            },
            MockResponse {
                response: "two".to_string(),
                tool_calls: Vec::new(),
            },
        ]);
        let mut runner = EvalRunner::new(vec![scenario()], provider);

        let result = runner.run_all();
        let provider = runner.into_executor();

        assert_eq!(result.passed, 1);
        assert_eq!(provider.recorded_interactions().len(), 2);
        assert_eq!(provider.recorded_interactions()[0].instruction, "first");
    }

    #[test]
    fn replay_requires_matching_scenario_and_instruction() {
        let provider = MockProvider::from_replay([RecordedInteraction {
            scenario: "other".to_string(),
            step_index: 0,
            instruction: "first".to_string(),
            observation: StepObservation {
                response: "one".to_string(),
                tool_calls: Vec::new(),
            },
        }]);
        let mut runner = EvalRunner::new(vec![scenario()], provider);

        let result = runner.run_all();

        assert_eq!(result.failed, 1);
        assert!(
            result.scenarios[0]
                .error
                .as_deref()
                .is_some_and(|error| error.contains("replay interaction mismatch"))
        );
    }
}
