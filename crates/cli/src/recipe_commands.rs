use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Context as _, Result, anyhow};
use clap::{Parser, Subcommand};
use recipe::{
    BuiltinRecipeSource, ExecutionContext, LocalRecipeSource, RecipeEngine, RecipeManifest,
};

#[derive(Parser, Debug)]
#[command(name = "recipe", about = "Manage Baymax recipes")]
struct RecipeArgs {
    #[arg(long, value_name = "DIR", global = true)]
    directory: Option<PathBuf>,
    #[command(subcommand)]
    command: RecipeCommand,
}

#[derive(Subcommand, Debug)]
enum RecipeCommand {
    #[command(about = "List available recipes")]
    List {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Search recipes by keyword")]
    Search {
        query: String,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Print a recipe YAML document")]
    Print { name: String },
    #[command(about = "Render recipe steps with provided variables")]
    Run {
        name: String,
        #[arg(long = "var", value_name = "KEY=VALUE")]
        variables: Vec<String>,
    },
}

pub fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let args = RecipeArgs::parse_from(args);
    run_command(args)
}

fn run_command(args: RecipeArgs) -> Result<()> {
    let mut stdout = io::stdout();
    run_command_with_writer(args, &mut stdout)
}

fn run_command_with_writer(args: RecipeArgs, output: &mut impl Write) -> Result<()> {
    let directory = args
        .directory
        .unwrap_or(env::current_dir().context("failed to read current directory")?);
    let engine = RecipeEngine::new()
        .with_source(LocalRecipeSource::new(directory))
        .with_source(BuiltinRecipeSource::baymax_defaults());

    match args.command {
        RecipeCommand::List { json } => {
            print_recipe_list(engine.discover_all()?, json, output)?;
        }
        RecipeCommand::Search { query, json } => {
            let query = query.to_ascii_lowercase();
            let recipes = engine
                .discover_all()?
                .into_iter()
                .filter(|recipe| recipe_matches_query(recipe, &query))
                .collect::<Vec<_>>();
            print_recipe_list(recipes, json, output)?;
        }
        RecipeCommand::Print { name } => {
            let recipe = engine.load(&name)?;
            write!(output, "{}", recipe.to_yaml()?)?;
        }
        RecipeCommand::Run { name, variables } => {
            let recipe = engine.load(&name)?;
            let mut context = ExecutionContext {
                variables: parse_variables(&variables)?,
                ..Default::default()
            };
            let recipe_output = engine.execute(&recipe, &mut context)?;
            for result in recipe_output.step_results {
                writeln!(output, "## {}", result.step_id)?;
                writeln!(output, "{}", result.prompt)?;
                if let Some(error) = result.error {
                    writeln!(output, "Error: {error}")?;
                }
            }
        }
    }

    Ok(())
}

fn print_recipe_list(
    recipes: Vec<RecipeManifest>,
    json: bool,
    output: &mut impl Write,
) -> Result<()> {
    if json {
        writeln!(output, "{}", serde_json::to_string_pretty(&recipes)?)?;
        return Ok(());
    }

    if recipes.is_empty() {
        writeln!(output, "No recipes found")?;
        return Ok(());
    }

    for recipe in recipes {
        writeln!(output, "{} - {}", recipe.name, recipe.description)?;
    }
    Ok(())
}

fn recipe_matches_query(recipe: &RecipeManifest, query: &str) -> bool {
    recipe.name.to_ascii_lowercase().contains(query)
        || recipe.description.to_ascii_lowercase().contains(query)
        || recipe
            .tags
            .iter()
            .any(|tag| tag.to_ascii_lowercase().contains(query))
}

fn parse_variables(variables: &[String]) -> Result<HashMap<String, String>> {
    let mut parsed = HashMap::new();
    for variable in variables {
        let Some((key, value)) = variable.split_once('=') else {
            return Err(anyhow!(
                "invalid variable `{variable}`; expected KEY=VALUE format"
            ));
        };
        if key.trim().is_empty() {
            return Err(anyhow!("variable key cannot be empty"));
        }
        parsed.insert(key.to_string(), value.to_string());
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tempfile::tempdir;

    const TEST_RECIPE: &str = r#"
title: Local Recipe
description: Local test recipe
prompt: Hello {{ name }}
tags:
  - local
parameters:
  - key: name
    input_type: string
    requirement: required
    description: Name
"#;

    fn write_test_recipe() -> tempfile::TempDir {
        let temp_dir = tempdir().unwrap();
        fs::write(temp_dir.path().join("local_recipe.yaml"), TEST_RECIPE).unwrap();
        temp_dir
    }

    #[test]
    fn parses_variables() {
        let variables = parse_variables(&["name=Baymax".to_string()]).unwrap();

        assert_eq!(variables.get("name"), Some(&"Baymax".to_string()));
    }

    #[test]
    fn rejects_malformed_variables() {
        let error = parse_variables(&["name".to_string()]).unwrap_err();

        assert!(error.to_string().contains("KEY=VALUE"));
    }

    #[test]
    fn matches_recipe_by_name_description_or_tag() {
        let manifest = RecipeManifest {
            name: "Release Risk".to_string(),
            description: "Assess release changes".to_string(),
            version: "1.0.0".to_string(),
            source: recipe::RecipeSourceType::Builtin,
            tags: vec!["release".to_string()],
            author: None,
            variables: Vec::new(),
        };

        assert!(recipe_matches_query(&manifest, "risk"));
        assert!(recipe_matches_query(&manifest, "changes"));
        assert!(recipe_matches_query(&manifest, "release"));
        assert!(!recipe_matches_query(&manifest, "missing"));
    }

    #[test]
    fn list_command_prints_local_recipes() {
        let temp_dir = write_test_recipe();
        let mut output = Vec::new();

        run_command_with_writer(
            RecipeArgs {
                directory: Some(temp_dir.path().to_path_buf()),
                command: RecipeCommand::List { json: false },
            },
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Local Recipe - Local test recipe"));
    }

    #[test]
    fn search_command_filters_recipes() {
        let temp_dir = write_test_recipe();
        let mut output = Vec::new();

        run_command_with_writer(
            RecipeArgs {
                directory: Some(temp_dir.path().to_path_buf()),
                command: RecipeCommand::Search {
                    query: "local".to_string(),
                    json: false,
                },
            },
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Local Recipe"));
    }

    #[test]
    fn print_command_outputs_recipe_yaml() {
        let temp_dir = write_test_recipe();
        let mut output = Vec::new();

        run_command_with_writer(
            RecipeArgs {
                directory: Some(temp_dir.path().to_path_buf()),
                command: RecipeCommand::Print {
                    name: "local_recipe".to_string(),
                },
            },
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("title: Local Recipe"));
    }

    #[test]
    fn run_command_renders_recipe_prompt() {
        let temp_dir = write_test_recipe();
        let mut output = Vec::new();

        run_command_with_writer(
            RecipeArgs {
                directory: Some(temp_dir.path().to_path_buf()),
                command: RecipeCommand::Run {
                    name: "local_recipe".to_string(),
                    variables: vec!["name=Baymax".to_string()],
                },
            },
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Hello Baymax"));
    }
}
