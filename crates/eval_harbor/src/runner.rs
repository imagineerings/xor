use crate::{
    AssertionEngine, EvalSuiteResult, ScenarioDefinition, ScenarioResult, StepObservation,
    StepResult,
};
use anyhow::{Context as _, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

pub trait ScenarioExecutor {
    fn execute_step(
        &mut self,
        scenario: &ScenarioDefinition,
        step_index: usize,
    ) -> Result<StepObservation>;
}

impl<T: ScenarioExecutor + ?Sized> ScenarioExecutor for Box<T> {
    fn execute_step(
        &mut self,
        scenario: &ScenarioDefinition,
        step_index: usize,
    ) -> Result<StepObservation> {
        self.as_mut().execute_step(scenario, step_index)
    }
}

#[derive(Clone, Debug)]
pub struct EvalRunner<E> {
    scenarios: Vec<ScenarioDefinition>,
    executor: E,
    assertion_engine: AssertionEngine,
}

impl<E> EvalRunner<E> {
    pub fn new(scenarios: Vec<ScenarioDefinition>, executor: E) -> Self {
        Self {
            scenarios,
            executor,
            assertion_engine: AssertionEngine::new(),
        }
    }

    pub fn scenarios(&self) -> &[ScenarioDefinition] {
        &self.scenarios
    }

    pub fn into_executor(self) -> E {
        self.executor
    }
}

impl<E: ScenarioExecutor> EvalRunner<E> {
    pub fn from_dir(scenarios_dir: impl AsRef<Path>, executor: E) -> Result<Self> {
        Ok(Self::new(load_scenarios(scenarios_dir)?, executor))
    }

    pub fn run_all(&mut self) -> EvalSuiteResult {
        let started_at = Instant::now();
        let mut scenarios = Vec::with_capacity(self.scenarios.len());

        for index in 0..self.scenarios.len() {
            scenarios.push(self.run_scenario_at(index));
        }

        let passed = scenarios.iter().filter(|scenario| scenario.passed).count();
        let total_scenarios = scenarios.len();
        EvalSuiteResult {
            total_scenarios,
            passed,
            failed: total_scenarios - passed,
            skipped: 0,
            scenarios,
            duration_millis: started_at.elapsed().as_millis(),
        }
    }

    pub fn run_scenario(&mut self, name: &str) -> Result<ScenarioResult> {
        let index = self
            .scenarios
            .iter()
            .position(|scenario| scenario.name == name)
            .with_context(|| format!("scenario {name:?} was not loaded"))?;
        Ok(self.run_scenario_at(index))
    }

    pub fn write_json_report(&mut self, output_path: impl AsRef<Path>) -> Result<EvalSuiteResult> {
        let result = self.run_all();
        let output_path = output_path.as_ref();
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create report directory {:?}", parent))?;
        }
        fs::write(output_path, result.to_json_string()?)
            .with_context(|| format!("failed to write eval report {:?}", output_path))?;
        Ok(result)
    }

    fn run_scenario_at(&mut self, index: usize) -> ScenarioResult {
        let started_at = Instant::now();
        let scenario = self.scenarios[index].clone();
        let mut observations = Vec::with_capacity(scenario.steps.len());
        let mut steps = Vec::with_capacity(scenario.steps.len());
        let mut scenario_error = None;

        for (step_index, step) in scenario.steps.iter().enumerate() {
            match self.executor.execute_step(&scenario, step_index) {
                Ok(observation) => {
                    let assertions = self.assertion_engine.evaluate_step(step, &observation);
                    let passed = assertions.iter().all(|assertion| assertion.passed);
                    observations.push(observation.clone());
                    steps.push(StepResult {
                        instruction: step.instruction.clone(),
                        observation,
                        assertions,
                        passed,
                    });
                }
                Err(error) => {
                    scenario_error = Some(error.to_string());
                    break;
                }
            }
        }

        let assertions = self
            .assertion_engine
            .evaluate_scenario(&scenario, &observations);
        let passed = scenario_error.is_none()
            && steps.iter().all(|step| step.passed)
            && assertions.iter().all(|assertion| assertion.passed);

        ScenarioResult {
            name: scenario.name,
            passed,
            steps,
            assertions,
            duration_millis: started_at.elapsed().as_millis(),
            error: scenario_error,
        }
    }
}

pub fn load_scenarios(scenarios_dir: impl AsRef<Path>) -> Result<Vec<ScenarioDefinition>> {
    let scenarios_dir = scenarios_dir.as_ref();
    let mut paths = Vec::new();
    for entry in fs::read_dir(scenarios_dir)
        .with_context(|| format!("failed to read scenarios directory {:?}", scenarios_dir))?
    {
        let path = entry
            .with_context(|| format!("failed to read entry in {:?}", scenarios_dir))?
            .path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
    }
    paths.sort();

    paths
        .into_iter()
        .map(load_scenario)
        .collect::<Result<Vec<_>>>()
}

fn load_scenario(path: PathBuf) -> Result<ScenarioDefinition> {
    let contents =
        fs::read_to_string(&path).with_context(|| format!("failed to read {:?}", path))?;
    serde_json::from_str(&contents).with_context(|| format!("failed to parse {:?}", path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExpectedOutcome, OutcomeCheck, ScenarioStep, ToolCallPattern};
    use std::collections::HashMap;

    #[derive(Default)]
    struct StubExecutor {
        responses: HashMap<String, StepObservation>,
    }

    impl ScenarioExecutor for StubExecutor {
        fn execute_step(
            &mut self,
            scenario: &ScenarioDefinition,
            step_index: usize,
        ) -> Result<StepObservation> {
            let key = format!("{}:{step_index}", scenario.name);
            self.responses
                .get(&key)
                .cloned()
                .with_context(|| format!("missing response for {key}"))
        }
    }

    #[test]
    fn runner_executes_scenarios_and_builds_report() {
        let scenario = ScenarioDefinition {
            name: "create file".to_string(),
            description: "writes a file".to_string(),
            steps: vec![ScenarioStep {
                instruction: "create README".to_string(),
                expected_tool_calls: vec![ToolCallPattern {
                    tool: "write_file".to_string(),
                    min_count: 1,
                }],
                expected_response_contains: vec!["created".to_string()],
            }],
            expected_outcomes: vec![ExpectedOutcome {
                check: OutcomeCheck::FinalOutput {
                    validator: "created".to_string(),
                },
                description: "final message says created".to_string(),
            }],
            tags: vec!["filesystem".to_string()],
        };
        let mut executor = StubExecutor::default();
        executor.responses.insert(
            "create file:0".to_string(),
            StepObservation {
                response: "created README".to_string(),
                tool_calls: vec!["write_file".to_string()],
            },
        );
        let mut runner = EvalRunner::new(vec![scenario], executor);

        let result = runner.run_all();

        assert_eq!(result.total_scenarios, 1);
        assert_eq!(result.passed, 1);
        assert!(result.scenarios[0].passed);
    }

    #[test]
    fn runner_loads_json_scenarios_and_writes_json_report() {
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let scenario_path = tempdir.path().join("scenario.json");
        fs::write(
            &scenario_path,
            r#"{
                "name": "loaded",
                "steps": [
                    {
                        "instruction": "answer",
                        "expected_response_contains": ["ok"]
                    }
                ],
                "expected_outcomes": [
                    {
                        "description": "contains ok",
                        "check": {
                            "kind": "response_contains",
                            "text": "ok"
                        }
                    }
                ]
            }"#,
        )
        .expect("write scenario");
        let mut executor = StubExecutor::default();
        executor.responses.insert(
            "loaded:0".to_string(),
            StepObservation {
                response: "ok".to_string(),
                tool_calls: Vec::new(),
            },
        );
        let mut runner = EvalRunner::from_dir(tempdir.path(), executor).expect("load scenarios");
        let report_path = tempdir.path().join("reports").join("report.json");

        let result = runner
            .write_json_report(&report_path)
            .expect("write json report");

        assert_eq!(result.passed, 1);
        let report = fs::read_to_string(report_path).expect("read report");
        assert!(report.contains("\"total_scenarios\": 1"));
    }
}
