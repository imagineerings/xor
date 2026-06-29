use std::collections::BTreeMap;

use anyhow::{Result, anyhow};

use crate::{
    ErrorPolicy, ExecutionContext, Recipe, RecipeManifest, RecipeOutput, RecipeStep, StepResult,
    TemplateEngine,
};

pub trait RecipeSource {
    fn discover(&self) -> Result<Vec<RecipeManifest>>;
    fn load(&self, name: &str) -> Result<Recipe>;
    fn priority(&self) -> u8;
}

#[derive(Default)]
pub struct RecipeEngine {
    sources: Vec<Box<dyn RecipeSource>>,
}

impl RecipeEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_source(mut self, source: impl RecipeSource + 'static) -> Self {
        self.sources.push(Box::new(source));
        self.sources
            .sort_by_key(|source| std::cmp::Reverse(source.priority()));
        self
    }

    pub fn discover_all(&self) -> Result<Vec<RecipeManifest>> {
        let mut manifests_by_name = BTreeMap::new();
        for source in &self.sources {
            for manifest in source.discover()? {
                manifests_by_name
                    .entry(manifest.name.to_lowercase())
                    .or_insert(manifest);
            }
        }
        Ok(manifests_by_name.into_values().collect())
    }

    pub fn load(&self, name: &str) -> Result<Recipe> {
        let mut errors = Vec::new();
        for source in &self.sources {
            match source.load(name) {
                Ok(recipe) => return Ok(recipe),
                Err(error) => errors.push(error.to_string()),
            }
        }

        Err(anyhow!(
            "recipe `{name}` was not found in any source: {}",
            errors.join("; ")
        ))
    }

    pub fn execute(&self, recipe: &Recipe, context: &mut ExecutionContext) -> Result<RecipeOutput> {
        self.execute_with(recipe, context, |_step, rendered_prompt| {
            Ok(rendered_prompt.to_string())
        })
    }

    pub fn execute_with(
        &self,
        recipe: &Recipe,
        context: &mut ExecutionContext,
        mut executor: impl FnMut(&RecipeStep, &str) -> Result<String>,
    ) -> Result<RecipeOutput> {
        let steps = recipe_steps(recipe);
        let mut step_results = Vec::new();
        let mut success = true;

        for (index, step) in steps.iter().enumerate() {
            context.current_step = index;
            let rendered_prompt = TemplateEngine::render(&step.prompt, &context.variables)?;

            match execute_step(step, &rendered_prompt, &mut executor) {
                Ok(output) => {
                    let result = StepResult {
                        step_id: step.id.clone(),
                        prompt: rendered_prompt,
                        output: Some(output),
                        success: true,
                        error: None,
                    };
                    context.step_results.push(result.clone());
                    step_results.push(result);
                }
                Err(error) => {
                    success = false;
                    let result = StepResult {
                        step_id: step.id.clone(),
                        prompt: rendered_prompt,
                        output: None,
                        success: false,
                        error: Some(error.to_string()),
                    };
                    context.step_results.push(result.clone());
                    step_results.push(result);

                    if matches!(step.error_policy, ErrorPolicy::Stop) {
                        break;
                    }
                }
            };
        }

        let completed_steps = step_results.iter().filter(|result| result.success).count();
        Ok(RecipeOutput {
            success,
            step_count: steps.len(),
            completed_steps,
            summary: format!(
                "completed {completed_steps} of {} recipe steps",
                steps.len()
            ),
            step_results,
        })
    }
}

fn execute_step(
    step: &RecipeStep,
    rendered_prompt: &str,
    executor: &mut impl FnMut(&RecipeStep, &str) -> Result<String>,
) -> Result<String> {
    match step.error_policy {
        ErrorPolicy::Retry => {
            let first_error = match executor(step, rendered_prompt) {
                Ok(output) => return Ok(output),
                Err(error) => error,
            };
            executor(step, rendered_prompt).map_err(|retry_error| {
                anyhow!(
                    "step failed after retry: {}; retry error: {}",
                    first_error,
                    retry_error
                )
            })
        }
        ErrorPolicy::Stop | ErrorPolicy::Skip | ErrorPolicy::Continue => {
            executor(step, rendered_prompt)
        }
    }
}

fn recipe_steps(recipe: &Recipe) -> Vec<RecipeStep> {
    if !recipe.steps.is_empty() {
        return recipe.steps.clone();
    }

    let mut steps = Vec::new();
    if let Some(instructions) = &recipe.instructions {
        steps.push(RecipeStep {
            id: "instructions".to_string(),
            prompt: instructions.clone(),
            tools: Vec::new(),
            condition: None,
            error_policy: Default::default(),
            wait_for_input: false,
        });
    }
    if let Some(prompt) = &recipe.prompt {
        steps.push(RecipeStep {
            id: "prompt".to_string(),
            prompt: prompt.clone(),
            tools: Vec::new(),
            condition: None,
            error_policy: Default::default(),
            wait_for_input: false,
        });
    }
    steps
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::{LocalRecipeSource, Recipe};

    use super::*;

    #[test]
    fn local_source_overrides_builtin_manifest() {
        let temp_dir = tempdir().unwrap();
        fs::write(
            temp_dir.path().join("release.yaml"),
            r#"
title: Release
description: Local release recipe
prompt: Local
"#,
        )
        .unwrap();
        let engine = RecipeEngine::new()
            .with_source(LocalRecipeSource::new(temp_dir.path()))
            .with_source(crate::sources::builtin::BuiltinRecipeSource::new(vec![
                crate::sources::builtin::BuiltinRecipe {
                    name: "release",
                    content: r#"
title: Release
description: Built-in release recipe
prompt: Built-in
"#,
                },
            ]));

        let manifests = engine.discover_all().unwrap();

        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].description, "Local release recipe");
    }

    #[test]
    fn executes_recipe_steps_with_variables() {
        let recipe = Recipe::from_yaml(
            r#"
title: Test
description: Test recipe
steps:
  - id: first
    prompt: Hello {{ name }}
  - id: second
    prompt: Goodbye {{ name }}
parameters:
  - key: name
    input_type: string
    requirement: required
    description: Name
"#,
        )
        .unwrap();
        let mut context = ExecutionContext::default();
        context
            .variables
            .insert("name".to_string(), "Baymax".to_string());

        let output = RecipeEngine::new().execute(&recipe, &mut context).unwrap();

        assert!(output.success);
        assert_eq!(output.completed_steps, 2);
        assert_eq!(output.step_results[0].prompt, "Hello Baymax");
        assert_eq!(context.current_step, 1);
    }

    #[test]
    fn stop_policy_aborts_on_executor_error() {
        let recipe = Recipe::from_yaml(
            r#"
title: Test
description: Test recipe
steps:
  - id: first
    prompt: First
    error_policy: stop
  - id: second
    prompt: Second
"#,
        )
        .unwrap();
        let mut context = ExecutionContext::default();

        let output = RecipeEngine::new()
            .execute_with(&recipe, &mut context, |step, _prompt| {
                if step.id == "first" {
                    anyhow::bail!("failed")
                }
                Ok("ok".to_string())
            })
            .unwrap();

        assert!(!output.success);
        assert_eq!(output.step_results.len(), 1);
        assert_eq!(output.completed_steps, 0);
    }

    #[test]
    fn continue_policy_records_error_and_keeps_going() {
        let recipe = Recipe::from_yaml(
            r#"
title: Test
description: Test recipe
steps:
  - id: first
    prompt: First
    error_policy: continue
  - id: second
    prompt: Second
"#,
        )
        .unwrap();
        let mut context = ExecutionContext::default();

        let output = RecipeEngine::new()
            .execute_with(&recipe, &mut context, |step, _prompt| {
                if step.id == "first" {
                    anyhow::bail!("failed")
                }
                Ok("ok".to_string())
            })
            .unwrap();

        assert!(!output.success);
        assert_eq!(output.step_results.len(), 2);
        assert_eq!(output.completed_steps, 1);
    }

    #[test]
    fn retry_policy_retries_once() {
        let recipe = Recipe::from_yaml(
            r#"
title: Test
description: Test recipe
steps:
  - id: first
    prompt: First
    error_policy: retry
"#,
        )
        .unwrap();
        let mut context = ExecutionContext::default();
        let mut attempts = 0;

        let output = RecipeEngine::new()
            .execute_with(&recipe, &mut context, |_step, _prompt| {
                attempts += 1;
                if attempts == 1 {
                    anyhow::bail!("failed")
                }
                Ok("ok".to_string())
            })
            .unwrap();

        assert!(output.success);
        assert_eq!(attempts, 2);
        assert_eq!(output.completed_steps, 1);
    }
}
