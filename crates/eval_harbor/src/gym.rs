use crate::{EvalRunner, EvalSuiteResult, ScenarioDefinition, ScenarioExecutor};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ModelEvalConfig {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub cost_per_eval: f64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvalTask {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub scenarios: Vec<ScenarioDefinition>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ModelComparison {
    pub models: Vec<ModelResult>,
    pub tasks: Vec<String>,
    pub summary: ComparisonSummary,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ModelResult {
    pub provider: String,
    pub model: String,
    pub task_results: Vec<TaskResult>,
    pub average_latency_millis: u128,
    pub total_cost: f64,
    pub overall_score: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TaskResult {
    pub task: String,
    pub model: String,
    pub suite_result: EvalSuiteResult,
    pub score: f64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ComparisonSummary {
    pub best_overall: Option<String>,
    pub best_latency: Option<String>,
    pub best_cost: Option<String>,
    pub rankings: Vec<(String, f64)>,
}

pub struct ModelGym<F> {
    configs: Vec<ModelEvalConfig>,
    tasks: Vec<EvalTask>,
    executor_factory: F,
}

impl<F> ModelGym<F>
where
    F: FnMut(&ModelEvalConfig, &EvalTask) -> Result<Box<dyn ScenarioExecutor>>,
{
    pub fn new(configs: Vec<ModelEvalConfig>, tasks: Vec<EvalTask>, executor_factory: F) -> Self {
        Self {
            configs,
            tasks,
            executor_factory,
        }
    }

    pub fn run_comparison(&mut self) -> Result<ModelComparison> {
        let mut models = Vec::with_capacity(self.configs.len());
        for config in self.configs.clone() {
            let mut task_results = Vec::with_capacity(self.tasks.len());
            for task in self.tasks.clone() {
                task_results.push(self.run_task_for_model(&config, task)?);
            }
            models.push(model_result(config, task_results));
        }

        let tasks = self.tasks.iter().map(|task| task.name.clone()).collect();
        let summary = comparison_summary(&models);
        Ok(ModelComparison {
            models,
            tasks,
            summary,
        })
    }

    pub fn run_task(&mut self, task_name: &str, model_names: &[&str]) -> Result<Vec<TaskResult>> {
        let task = self
            .tasks
            .iter()
            .find(|task| task.name == task_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("eval task {task_name:?} was not configured"))?;
        let configs = self
            .configs
            .iter()
            .filter(|config| model_names.iter().any(|name| *name == config.model))
            .cloned()
            .collect::<Vec<_>>();

        configs
            .into_iter()
            .map(|config| self.run_task_for_model(&config, task.clone()))
            .collect()
    }

    fn run_task_for_model(
        &mut self,
        config: &ModelEvalConfig,
        task: EvalTask,
    ) -> Result<TaskResult> {
        let executor = (self.executor_factory)(config, &task)?;
        let mut runner = EvalRunner::new(task.scenarios, executor);
        let suite_result = runner.run_all();
        let score = pass_ratio(&suite_result);
        Ok(TaskResult {
            task: task.name,
            model: config.model.clone(),
            suite_result,
            score,
        })
    }
}

fn model_result(config: ModelEvalConfig, task_results: Vec<TaskResult>) -> ModelResult {
    let total_scenarios = task_results
        .iter()
        .map(|result| result.suite_result.total_scenarios)
        .sum::<usize>();
    let total_latency = task_results
        .iter()
        .map(|result| result.suite_result.duration_millis)
        .sum::<u128>();
    let passed = task_results
        .iter()
        .map(|result| result.suite_result.passed)
        .sum::<usize>();
    let average_latency_millis = if total_scenarios == 0 {
        0
    } else {
        total_latency / total_scenarios as u128
    };
    let overall_score = if total_scenarios == 0 {
        0.0
    } else {
        passed as f64 / total_scenarios as f64
    };

    ModelResult {
        provider: config.provider,
        model: config.model,
        task_results,
        average_latency_millis,
        total_cost: config.cost_per_eval * total_scenarios as f64,
        overall_score,
    }
}

fn comparison_summary(models: &[ModelResult]) -> ComparisonSummary {
    let best_overall = models
        .iter()
        .max_by(|left, right| left.overall_score.total_cmp(&right.overall_score))
        .map(|result| result.model.clone());
    let best_latency = models
        .iter()
        .min_by_key(|result| result.average_latency_millis)
        .map(|result| result.model.clone());
    let best_cost = models
        .iter()
        .min_by(|left, right| left.total_cost.total_cmp(&right.total_cost))
        .map(|result| result.model.clone());
    let mut rankings = models
        .iter()
        .map(|result| (result.model.clone(), result.overall_score))
        .collect::<Vec<_>>();
    rankings.sort_by(|left, right| right.1.total_cmp(&left.1));

    ComparisonSummary {
        best_overall,
        best_latency,
        best_cost,
        rankings,
    }
}

fn pass_ratio(result: &EvalSuiteResult) -> f64 {
    if result.total_scenarios == 0 {
        0.0
    } else {
        result.passed as f64 / result.total_scenarios as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockProvider, MockResponse, ScenarioStep};

    fn task() -> EvalTask {
        EvalTask {
            name: "coding".to_string(),
            description: String::new(),
            scenarios: vec![ScenarioDefinition {
                name: "simple".to_string(),
                description: String::new(),
                steps: vec![ScenarioStep {
                    instruction: "answer".to_string(),
                    expected_tool_calls: Vec::new(),
                    expected_response_contains: vec!["ok".to_string()],
                }],
                expected_outcomes: Vec::new(),
                tags: Vec::new(),
            }],
        }
    }

    #[test]
    fn compares_models_by_quality_latency_and_cost() {
        let configs = vec![
            ModelEvalConfig {
                provider: "mock".to_string(),
                model: "good".to_string(),
                cost_per_eval: 0.2,
            },
            ModelEvalConfig {
                provider: "mock".to_string(),
                model: "bad".to_string(),
                cost_per_eval: 0.1,
            },
        ];
        let mut gym = ModelGym::new(configs, vec![task()], |config, _task| {
            let response = if config.model == "good" { "ok" } else { "nope" };
            Ok(Box::new(MockProvider::from_responses([MockResponse {
                response: response.to_string(),
                tool_calls: Vec::new(),
            }])) as Box<dyn ScenarioExecutor>)
        });

        let comparison = gym.run_comparison().expect("run comparison");

        assert_eq!(comparison.models.len(), 2);
        assert_eq!(comparison.summary.best_overall.as_deref(), Some("good"));
        assert_eq!(comparison.summary.best_cost.as_deref(), Some("bad"));
        assert_eq!(comparison.summary.rankings[0].0, "good");
    }
}
