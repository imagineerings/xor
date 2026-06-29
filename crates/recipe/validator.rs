use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    Recipe, RecipeParameterInputType, RecipeParameterRequirement, TemplateEngine, parse_recipe_yaml,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: Severity,
}

impl ValidationError {
    fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: Severity::Error,
        }
    }
}

pub struct RecipeValidator;

impl RecipeValidator {
    pub fn validate_yaml(content: &str) -> anyhow::Result<Recipe> {
        let recipe = parse_recipe_yaml::<Recipe>(content)?;
        let errors = Self::validate(&recipe);
        if errors.iter().any(|error| error.severity == Severity::Error) {
            let message = errors
                .iter()
                .map(|error| format!("{}: {}", error.field, error.message))
                .collect::<Vec<_>>()
                .join("; ");
            anyhow::bail!("recipe validation failed: {message}");
        }
        Ok(recipe)
    }

    pub fn validate(recipe: &Recipe) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        if recipe.title.trim().is_empty() {
            errors.push(ValidationError::error("title", "title is required"));
        }

        if recipe.description.trim().is_empty() {
            errors.push(ValidationError::error(
                "description",
                "description is required",
            ));
        }

        let has_prompt = recipe
            .prompt
            .as_ref()
            .is_some_and(|prompt| !prompt.trim().is_empty());
        let has_instructions = recipe
            .instructions
            .as_ref()
            .is_some_and(|instructions| !instructions.trim().is_empty());
        if !has_prompt && !has_instructions && recipe.steps.is_empty() {
            errors.push(ValidationError::error(
                "prompt",
                "recipe must specify prompt, instructions, or at least one step",
            ));
        }

        validate_parameters(recipe, &mut errors);
        validate_template_variables(recipe, &mut errors);
        validate_steps(recipe, &mut errors);

        errors
    }
}

fn validate_parameters(recipe: &Recipe, errors: &mut Vec<ValidationError>) {
    let mut parameter_keys = BTreeSet::new();
    for parameter in &recipe.parameters {
        if parameter.key.trim().is_empty() {
            errors.push(ValidationError::error(
                "parameters.key",
                "parameter key is required",
            ));
        }

        if !parameter_keys.insert(parameter.key.clone()) {
            errors.push(ValidationError::error(
                format!("parameters.{}", parameter.key),
                "parameter key must be unique",
            ));
        }

        if parameter.description.trim().is_empty() {
            errors.push(ValidationError::error(
                format!("parameters.{}", parameter.key),
                "parameter description is required",
            ));
        }

        if matches!(parameter.requirement, RecipeParameterRequirement::Optional)
            && parameter.default.is_none()
        {
            errors.push(ValidationError::error(
                format!("parameters.{}", parameter.key),
                "optional parameters must provide a default value",
            ));
        }

        if matches!(parameter.input_type, RecipeParameterInputType::File)
            && parameter.default.is_some()
        {
            errors.push(ValidationError::error(
                format!("parameters.{}", parameter.key),
                "file parameters cannot provide default values",
            ));
        }
    }
}

fn validate_template_variables(recipe: &Recipe, errors: &mut Vec<ValidationError>) {
    let available_variables = recipe
        .parameters
        .iter()
        .map(|parameter| parameter.key.clone())
        .collect::<BTreeSet<_>>();

    validate_template_field(
        "instructions",
        recipe.instructions.as_deref(),
        &available_variables,
        errors,
    );
    validate_template_field(
        "prompt",
        recipe.prompt.as_deref(),
        &available_variables,
        errors,
    );

    for step in &recipe.steps {
        validate_template_field(
            format!("steps.{}.prompt", step.id),
            Some(step.prompt.as_str()),
            &available_variables,
            errors,
        );
    }
}

fn validate_template_field(
    field: impl Into<String>,
    value: Option<&str>,
    available_variables: &BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    let Some(value) = value else {
        return;
    };
    let missing_variables = TemplateEngine::validate_template(value, available_variables);
    if missing_variables.is_empty() {
        return;
    }

    errors.push(ValidationError::error(
        field,
        format!(
            "missing parameter definitions for template variables: {}",
            missing_variables.join(", ")
        ),
    ));
}

fn validate_steps(recipe: &Recipe, errors: &mut Vec<ValidationError>) {
    let mut step_ids = BTreeSet::new();
    for step in &recipe.steps {
        if step.id.trim().is_empty() {
            errors.push(ValidationError::error("steps.id", "step id is required"));
        }

        if !step_ids.insert(step.id.clone()) {
            errors.push(ValidationError::error(
                format!("steps.{}", step.id),
                "step id must be unique",
            ));
        }

        if step.prompt.trim().is_empty() {
            errors.push(ValidationError::error(
                format!("steps.{}.prompt", step.id),
                "step prompt is required",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_recipe() {
        let recipe = RecipeValidator::validate_yaml(
            r#"
title: Valid
description: A valid recipe
instructions: Hello {{ name }}
parameters:
  - key: name
    input_type: string
    requirement: required
    description: Name to greet
"#,
        )
        .unwrap();

        assert_eq!(recipe.title, "Valid");
    }

    #[test]
    fn rejects_recipe_without_prompt_instructions_or_steps() {
        let error = RecipeValidator::validate_yaml(
            r#"
title: Invalid
description: Missing instructions
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("prompt"));
    }

    #[test]
    fn reports_missing_template_parameter() {
        let recipe = parse_recipe_yaml::<Recipe>(
            r#"
title: Invalid
description: Missing parameter
instructions: Hello {{ name }}
"#,
        )
        .unwrap();

        let errors = RecipeValidator::validate(&recipe);

        assert!(
            errors
                .iter()
                .any(|error| { error.field == "instructions" && error.message.contains("name") })
        );
    }

    #[test]
    fn rejects_optional_parameter_without_default() {
        let error = RecipeValidator::validate_yaml(
            r#"
title: Invalid
description: Bad optional parameter
instructions: Hello
parameters:
  - key: name
    input_type: string
    requirement: optional
    description: Optional name
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("optional parameters"));
    }
}
