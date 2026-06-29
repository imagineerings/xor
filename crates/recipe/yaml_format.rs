use anyhow::{Context as _, Result};
use serde::{Serialize, de::DeserializeOwned};

pub const MULTILINE_RECIPE_FIELDS: &[&str] = &["prompt", "instructions"];

pub fn parse_recipe_yaml<T>(content: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let yaml_value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(content).context("failed to parse recipe YAML")?;

    if let Some(nested_recipe) = yaml_value.get("recipe") {
        serde_yaml_ng::from_value(nested_recipe.clone())
            .context("failed to parse nested recipe YAML")
    } else {
        serde_yaml_ng::from_value(yaml_value).context("failed to parse recipe YAML")
    }
}

pub fn format_recipe_yaml<T>(recipe: &T) -> Result<String>
where
    T: Serialize,
{
    let yaml = serde_yaml_ng::to_string(recipe).context("failed to serialize recipe YAML")?;
    Ok(reformat_fields_with_multiline_values(
        &yaml,
        MULTILINE_RECIPE_FIELDS,
    ))
}

pub fn reformat_fields_with_multiline_values(yaml: &str, multiline_fields: &[&str]) -> String {
    let mut result = String::new();

    for line in yaml.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            result.push('\n');
            continue;
        }

        let indent = line.len() - trimmed.len();
        let indent_string = " ".repeat(indent);
        let matched_field = multiline_fields
            .iter()
            .find(|field| trimmed.starts_with(&format!("{field}: ")));

        if let Some(field) = matched_field
            && let Some((_, raw_value)) = trimmed.split_once(": ")
            && raw_value.contains("\\n")
        {
            let value = raw_value
                .trim_matches('"')
                .replace("\\\"", "\"")
                .replace("\\\\n", "\\n");

            result.push_str(&format!("{indent_string}{field}: |\n"));
            for line in value.split("\\n") {
                result.push_str(&format!("{indent_string}  {line}\n"));
            }
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Recipe;

    #[test]
    fn reports_parse_context() {
        let error = parse_recipe_yaml::<Recipe>("title: [").unwrap_err();
        assert!(error.to_string().contains("failed to parse recipe YAML"));
    }

    #[test]
    fn converts_multiline_prompt_to_literal_block() {
        let yaml = "version: \"1.0\"\nprompt: \"line1\\\\nline2\"";
        let expected = "version: \"1.0\"\nprompt: |\n  line1\n  line2\n";

        let result = reformat_fields_with_multiline_values(yaml, &["prompt"]);
        assert_eq!(result, expected);
    }

    #[test]
    fn preserves_unlisted_multiline_fields() {
        let yaml = "prompt: \"line1\\\\nline2\"\nnotes: \"note1\\\\nnote2\"";
        let expected = "prompt: |\n  line1\n  line2\nnotes: \"note1\\\\nnote2\"\n";

        let result = reformat_fields_with_multiline_values(yaml, &["prompt"]);
        assert_eq!(result, expected);
    }
}
