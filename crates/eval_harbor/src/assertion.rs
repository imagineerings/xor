use crate::{
    AssertionResult, ExpectedOutcome, OutcomeCheck, ScenarioDefinition, ScenarioStep,
    StepObservation, ToolCallPattern,
};
use regex::Regex;

#[derive(Clone, Debug, Default)]
pub struct AssertionEngine;

impl AssertionEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate_step(
        &self,
        step: &ScenarioStep,
        observation: &StepObservation,
    ) -> Vec<AssertionResult> {
        let tool_call_assertions = step
            .expected_tool_calls
            .iter()
            .map(|pattern| self.evaluate_tool_call(pattern, observation));

        let response_assertions = step
            .expected_response_contains
            .iter()
            .map(|text| self.evaluate_response_contains(text, observation));

        tool_call_assertions.chain(response_assertions).collect()
    }

    pub fn evaluate_scenario(
        &self,
        scenario: &ScenarioDefinition,
        observations: &[StepObservation],
    ) -> Vec<AssertionResult> {
        scenario
            .expected_outcomes
            .iter()
            .map(|outcome| self.evaluate_expected_outcome(outcome, observations))
            .collect()
    }

    fn evaluate_expected_outcome(
        &self,
        outcome: &ExpectedOutcome,
        observations: &[StepObservation],
    ) -> AssertionResult {
        match &outcome.check {
            OutcomeCheck::ToolCalled { tool, min_count } => {
                let count = count_tool_calls(observations, tool);
                AssertionResult {
                    assertion: outcome.description.clone(),
                    passed: count >= *min_count,
                    actual: count.to_string(),
                    expected: format!("at least {min_count} call(s) to {tool}"),
                }
            }
            OutcomeCheck::ResponseContains { text } => {
                let found = observations
                    .iter()
                    .any(|observation| observation.response.contains(text));
                AssertionResult {
                    assertion: outcome.description.clone(),
                    passed: found,
                    actual: joined_responses(observations),
                    expected: format!("response containing {text:?}"),
                }
            }
            OutcomeCheck::ResponseMatches { pattern } => match Regex::new(pattern) {
                Ok(regex) => {
                    let found = observations
                        .iter()
                        .any(|observation| regex.is_match(&observation.response));
                    AssertionResult {
                        assertion: outcome.description.clone(),
                        passed: found,
                        actual: joined_responses(observations),
                        expected: format!("response matching {pattern:?}"),
                    }
                }
                Err(error) => AssertionResult {
                    assertion: outcome.description.clone(),
                    passed: false,
                    actual: error.to_string(),
                    expected: format!("valid regex {pattern:?}"),
                },
            },
            OutcomeCheck::FinalOutput { validator } => {
                let final_response = observations
                    .last()
                    .map(|observation| observation.response.as_str())
                    .unwrap_or_default();
                AssertionResult {
                    assertion: outcome.description.clone(),
                    passed: final_response.contains(validator),
                    actual: final_response.to_string(),
                    expected: format!("final output accepted by {validator:?}"),
                }
            }
        }
    }

    fn evaluate_tool_call(
        &self,
        pattern: &ToolCallPattern,
        observation: &StepObservation,
    ) -> AssertionResult {
        let count = observation
            .tool_calls
            .iter()
            .filter(|tool_call| *tool_call == &pattern.tool)
            .count();
        AssertionResult {
            assertion: format!("tool called: {}", pattern.tool),
            passed: count >= pattern.min_count,
            actual: count.to_string(),
            expected: format!("at least {} call(s)", pattern.min_count),
        }
    }

    fn evaluate_response_contains(
        &self,
        text: &str,
        observation: &StepObservation,
    ) -> AssertionResult {
        AssertionResult {
            assertion: format!("response contains: {text}"),
            passed: observation.response.contains(text),
            actual: observation.response.clone(),
            expected: text.to_string(),
        }
    }
}

fn count_tool_calls(observations: &[StepObservation], tool: &str) -> usize {
    observations
        .iter()
        .flat_map(|observation| observation.tool_calls.iter())
        .filter(|tool_call| tool_call.as_str() == tool)
        .count()
}

fn joined_responses(observations: &[StepObservation]) -> String {
    observations
        .iter()
        .map(|observation| observation.response.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_all_scenario_outcomes() {
        let scenario = ScenarioDefinition {
            name: "tool use".to_string(),
            description: String::new(),
            steps: Vec::new(),
            expected_outcomes: vec![
                ExpectedOutcome {
                    check: OutcomeCheck::ToolCalled {
                        tool: "shell".to_string(),
                        min_count: 2,
                    },
                    description: "shell used twice".to_string(),
                },
                ExpectedOutcome {
                    check: OutcomeCheck::ResponseMatches {
                        pattern: "done|complete".to_string(),
                    },
                    description: "completion response".to_string(),
                },
            ],
            tags: Vec::new(),
        };
        let observations = vec![
            StepObservation {
                response: "working".to_string(),
                tool_calls: vec!["shell".to_string()],
            },
            StepObservation {
                response: "done".to_string(),
                tool_calls: vec!["shell".to_string()],
            },
        ];

        let results = AssertionEngine::new().evaluate_scenario(&scenario, &observations);

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|result| result.passed));
    }

    #[test]
    fn invalid_regex_fails_the_assertion() {
        let scenario = ScenarioDefinition {
            name: "bad regex".to_string(),
            description: String::new(),
            steps: Vec::new(),
            expected_outcomes: vec![ExpectedOutcome {
                check: OutcomeCheck::ResponseMatches {
                    pattern: "[".to_string(),
                },
                description: "valid pattern".to_string(),
            }],
            tags: Vec::new(),
        };

        let results = AssertionEngine::new().evaluate_scenario(&scenario, &[]);

        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
    }
}
