use std::collections::{BTreeSet, HashMap};

use regex::Regex;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TemplateError {
    #[error("missing template variable `{0}`")]
    MissingVariable(String),
}

pub struct TemplateEngine;

impl TemplateEngine {
    pub fn render(
        template: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, TemplateError> {
        let expression = variable_expression();
        let mut rendered = String::with_capacity(template.len());
        let mut previous_end = 0;

        for capture in expression.captures_iter(template) {
            let Some(matched) = capture.get(0) else {
                continue;
            };
            let Some(variable_match) = capture.get(1) else {
                continue;
            };
            let variable_name = variable_match.as_str();
            let value = variables
                .get(variable_name)
                .ok_or_else(|| TemplateError::MissingVariable(variable_name.to_string()))?;

            rendered.push_str(&template[previous_end..matched.start()]);
            rendered.push_str(value);
            previous_end = matched.end();
        }

        rendered.push_str(&template[previous_end..]);
        Ok(rendered)
    }

    pub fn extract_variables(template: &str) -> Vec<String> {
        let expression = variable_expression();
        expression
            .captures_iter(template)
            .filter_map(|capture| capture.get(1).map(|matched| matched.as_str().to_string()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn validate_template(
        template: &str,
        available_variables: &BTreeSet<String>,
    ) -> Vec<String> {
        Self::extract_variables(template)
            .into_iter()
            .filter(|variable| !available_variables.contains(variable))
            .collect()
    }
}

fn variable_expression() -> Regex {
    Regex::new(r"\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}\}")
        .expect("template variable regex should compile")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_simple_variables() {
        let variables = HashMap::from([
            ("greeting".to_string(), "Hello".to_string()),
            ("name".to_string(), "Baymax".to_string()),
        ]);

        let rendered = TemplateEngine::render("{{ greeting }}, {{name}}!", &variables).unwrap();

        assert_eq!(rendered, "Hello, Baymax!");
    }

    #[test]
    fn reports_missing_variable() {
        let error = TemplateEngine::render("Hello {{ name }}", &HashMap::new()).unwrap_err();

        assert_eq!(error, TemplateError::MissingVariable("name".to_string()));
    }

    #[test]
    fn extracts_unique_variables_in_stable_order() {
        let variables = TemplateEngine::extract_variables("{{ zed }} {{ baymax }} {{ zed }}");

        assert_eq!(variables, vec!["baymax".to_string(), "zed".to_string()]);
    }

    #[test]
    fn ignores_complex_goose_placeholders() {
        let variables = TemplateEngine::extract_variables("{{ hf model org }} {{ valid_name }}");

        assert_eq!(variables, vec!["valid_name".to_string()]);
    }
}
