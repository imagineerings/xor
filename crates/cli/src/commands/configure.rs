use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow};
use clap::Parser;
use serde_json::{Map, Value, json};

#[derive(Parser, Debug)]
#[command(name = "configure", about = "Configure Sim agent settings")]
struct ConfigureArgs {
    #[arg(long, value_name = "FILE", hide = true)]
    settings_file: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardInputType {
    Text { secret: bool },
    Confirm,
    Select { options: Vec<String> },
    File { must_exist: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WizardStep {
    pub id: String,
    pub prompt: String,
    pub input_type: WizardInputType,
    pub default: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardAdvance {
    Advanced,
    Complete(ConfigurationAnswers),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigurationAnswers {
    values: HashMap<String, String>,
}

impl ConfigurationAnswers {
    pub fn get(&self, id: &str) -> Option<&str> {
        self.values.get(id).map(String::as_str)
    }

    pub fn insert(&mut self, id: String, value: String) {
        self.values.insert(id, value);
    }
}

#[derive(Debug, Clone)]
pub struct ConfigWizard {
    steps: Vec<WizardStep>,
    current_step: usize,
    answers: ConfigurationAnswers,
}

impl ConfigWizard {
    pub fn provider_setup() -> Self {
        Self::new(vec![
            WizardStep {
                id: "provider".to_string(),
                prompt: "Provider".to_string(),
                input_type: WizardInputType::Select {
                    options: vec![
                        "sim.dev".to_string(),
                        "openai".to_string(),
                        "anthropic".to_string(),
                        "ollama".to_string(),
                    ],
                },
                default: Some("sim.dev".to_string()),
                required: true,
            },
            WizardStep {
                id: "model".to_string(),
                prompt: "Model".to_string(),
                input_type: WizardInputType::Text { secret: false },
                default: Some("claude-sonnet-4".to_string()),
                required: true,
            },
            WizardStep {
                id: "api_key".to_string(),
                prompt: "API key".to_string(),
                input_type: WizardInputType::Text { secret: true },
                default: None,
                required: false,
            },
            WizardStep {
                id: "extension_path".to_string(),
                prompt: "Extension or MCP config path".to_string(),
                input_type: WizardInputType::File { must_exist: true },
                default: None,
                required: false,
            },
        ])
    }

    pub fn new(steps: Vec<WizardStep>) -> Self {
        Self {
            steps,
            current_step: 0,
            answers: ConfigurationAnswers::default(),
        }
    }

    pub fn current_step(&self) -> Option<&WizardStep> {
        self.steps.get(self.current_step)
    }

    pub fn current_prompt(&self) -> Option<String> {
        self.current_step().map(|step| {
            let default = step
                .default
                .as_ref()
                .map(|default| format!(" [{default}]"))
                .unwrap_or_default();
            format!("{}{}: ", step.prompt, default)
        })
    }

    pub fn answers(&self) -> &ConfigurationAnswers {
        &self.answers
    }

    pub fn is_complete(&self) -> bool {
        self.current_step >= self.steps.len()
    }

    pub fn masked_answer(step: &WizardStep, answer: &str) -> String {
        match step.input_type {
            WizardInputType::Text { secret: true } if !answer.is_empty() => "********".to_string(),
            _ => answer.to_string(),
        }
    }

    pub fn process_answer(&mut self, answer: &str) -> Result<WizardAdvance> {
        let Some(step) = self.current_step().cloned() else {
            return Ok(WizardAdvance::Complete(self.answers.clone()));
        };

        let value = normalize_answer(&step, answer)?;
        if !value.is_empty() {
            self.answers.insert(step.id, value);
        }
        self.current_step += 1;

        if self.is_complete() {
            Ok(WizardAdvance::Complete(self.answers.clone()))
        } else {
            Ok(WizardAdvance::Advanced)
        }
    }
}

pub trait WizardPersistence {
    fn persist(&self, answers: &ConfigurationAnswers) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct SimSettingsPersistence {
    path: PathBuf,
}

impl SimSettingsPersistence {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn user_settings_file() -> Self {
        Self::new(paths::settings_file().clone())
    }
}

impl WizardPersistence for SimSettingsPersistence {
    fn persist(&self, answers: &ConfigurationAnswers) -> Result<()> {
        let mut settings = read_json_file(&self.path)?;
        merge_wizard_answers(&mut settings, answers)?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating settings directory {}", parent.display()))?;
        }
        let serialized = serde_json::to_string_pretty(&settings)?;
        fs::write(&self.path, format!("{serialized}\n"))
            .with_context(|| format!("writing settings file {}", self.path.display()))?;
        Ok(())
    }
}

pub fn run_wizard_with_io(
    wizard: &mut ConfigWizard,
    input: &mut impl BufRead,
    output: &mut impl Write,
    persistence: &impl WizardPersistence,
) -> Result<ConfigurationAnswers> {
    loop {
        let Some(prompt) = wizard.current_prompt() else {
            let answers = wizard.answers().clone();
            persistence.persist(&answers)?;
            return Ok(answers);
        };

        write!(output, "{prompt}")?;
        output.flush()?;

        let mut line = String::new();
        let bytes = input.read_line(&mut line)?;
        if bytes == 0 {
            return Err(anyhow!(
                "configuration wizard input ended before completion"
            ));
        }

        match wizard.process_answer(line.trim_end())? {
            WizardAdvance::Advanced => {}
            WizardAdvance::Complete(answers) => {
                persistence.persist(&answers)?;
                writeln!(output, "Configuration saved.")?;
                return Ok(answers);
            }
        }
    }
}

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let args = ConfigureArgs::try_parse_from(args)?;
    let persistence = args
        .settings_file
        .map(SimSettingsPersistence::new)
        .unwrap_or_else(SimSettingsPersistence::user_settings_file);
    let mut wizard = ConfigWizard::provider_setup();
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout();
    run_wizard_with_io(&mut wizard, &mut input, &mut output, &persistence)?;
    Ok(())
}

fn normalize_answer(step: &WizardStep, answer: &str) -> Result<String> {
    let value = if answer.trim().is_empty() {
        step.default.clone().unwrap_or_default()
    } else {
        answer.trim().to_string()
    };

    if step.required && value.is_empty() {
        return Err(anyhow!("{} is required", step.prompt));
    }

    match &step.input_type {
        WizardInputType::Text { .. } => Ok(value),
        WizardInputType::Confirm => normalize_confirm_answer(&value),
        WizardInputType::Select { options } => {
            if options.iter().any(|option| option == &value) {
                Ok(value)
            } else {
                Err(anyhow!(
                    "invalid selection `{value}`; expected one of: {}",
                    options.join(", ")
                ))
            }
        }
        WizardInputType::File { must_exist } => {
            if value.is_empty() {
                return Ok(value);
            }
            let path = Path::new(&value);
            if *must_exist && !path.exists() {
                return Err(anyhow!("file does not exist: {}", path.display()));
            }
            Ok(value)
        }
    }
}

fn normalize_confirm_answer(answer: &str) -> Result<String> {
    match answer.to_ascii_lowercase().as_str() {
        "y" | "yes" | "true" => Ok("true".to_string()),
        "n" | "no" | "false" => Ok("false".to_string()),
        value => Err(anyhow!(
            "invalid confirmation `{value}`; expected yes or no"
        )),
    }
}

fn read_json_file(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("reading settings file {}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    serde_json::from_str(&content)
        .with_context(|| format!("parsing settings file {}", path.display()))
}

fn merge_wizard_answers(settings: &mut Value, answers: &ConfigurationAnswers) -> Result<()> {
    let agent = ensure_child_object(settings, "agent")?;

    if let (Some(provider), Some(model)) = (answers.get("provider"), answers.get("model")) {
        agent.insert(
            "default_model".to_string(),
            json!({
                "provider": provider,
                "model": model,
            }),
        );
    }

    if let Some(extension_path) = answers.get("extension_path")
        && !extension_path.is_empty()
    {
        let onboarding = ensure_child_object_from_map(agent, "cli_onboarding")?;
        onboarding.insert("extension_path".to_string(), json!(extension_path));
    }

    if let Some(api_key) = answers.get("api_key")
        && !api_key.is_empty()
    {
        let onboarding = ensure_child_object_from_map(agent, "cli_onboarding")?;
        onboarding.insert("api_key_configured".to_string(), json!(true));
    }

    Ok(())
}

fn ensure_object(value: &mut Value) -> Result<&mut Map<String, Value>> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value
        .as_object_mut()
        .ok_or_else(|| anyhow!("failed to prepare settings object"))
}

fn ensure_child_object<'a>(value: &'a mut Value, key: &str) -> Result<&'a mut Map<String, Value>> {
    let object = ensure_object(value)?;
    ensure_child_object_from_map(object, key)
}

fn ensure_child_object_from_map<'a>(
    object: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>> {
    let child = object
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    ensure_object(child)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn validates_select_confirm_and_file_inputs() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let config_path = temp_dir.path().join("mcp.json");
        fs::write(&config_path, "{}")?;
        let mut wizard = ConfigWizard::new(vec![
            WizardStep {
                id: "provider".to_string(),
                prompt: "Provider".to_string(),
                input_type: WizardInputType::Select {
                    options: vec!["openai".to_string()],
                },
                default: None,
                required: true,
            },
            WizardStep {
                id: "confirm".to_string(),
                prompt: "Confirm".to_string(),
                input_type: WizardInputType::Confirm,
                default: Some("yes".to_string()),
                required: true,
            },
            WizardStep {
                id: "file".to_string(),
                prompt: "File".to_string(),
                input_type: WizardInputType::File { must_exist: true },
                default: None,
                required: true,
            },
        ]);

        assert!(wizard.process_answer("missing").is_err());
        assert_eq!(wizard.process_answer("openai")?, WizardAdvance::Advanced);
        assert_eq!(wizard.process_answer("")?, WizardAdvance::Advanced);
        let complete = wizard.process_answer(&config_path.to_string_lossy())?;

        let WizardAdvance::Complete(answers) = complete else {
            return Err(anyhow!("wizard should be complete"));
        };
        assert_eq!(answers.get("provider"), Some("openai"));
        assert_eq!(answers.get("confirm"), Some("true"));
        Ok(())
    }

    #[test]
    fn masks_secret_answers_for_display() {
        let step = WizardStep {
            id: "api_key".to_string(),
            prompt: "API key".to_string(),
            input_type: WizardInputType::Text { secret: true },
            default: None,
            required: false,
        };

        assert_eq!(ConfigWizard::masked_answer(&step, "secret"), "********");
    }

    #[test]
    fn persists_provider_model_to_settings_json() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let settings_path = temp_dir.path().join("settings.json");
        fs::write(&settings_path, r#"{ "agent": { "dock": "left" } }"#)?;
        let persistence = SimSettingsPersistence::new(settings_path.clone());
        let mut answers = ConfigurationAnswers::default();
        answers.insert("provider".to_string(), "openai".to_string());
        answers.insert("model".to_string(), "gpt-5".to_string());

        persistence.persist(&answers)?;

        let settings: Value = serde_json::from_str(&fs::read_to_string(settings_path)?)?;
        assert_eq!(settings["agent"]["dock"], "left");
        assert_eq!(settings["agent"]["default_model"]["provider"], "openai");
        assert_eq!(settings["agent"]["default_model"]["model"], "gpt-5");
        Ok(())
    }

    #[test]
    fn runs_wizard_with_io_and_persists_answers() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let extension_path = temp_dir.path().join("extension.json");
        fs::write(&extension_path, "{}")?;
        let settings_path = temp_dir.path().join("settings.json");
        let persistence = SimSettingsPersistence::new(settings_path.clone());
        let mut wizard = ConfigWizard::provider_setup();
        let mut input = Cursor::new(format!("openai\ngpt-5\n\n{}\n", extension_path.display()));
        let mut output = Vec::new();

        let answers = run_wizard_with_io(&mut wizard, &mut input, &mut output, &persistence)?;

        assert_eq!(answers.get("provider"), Some("openai"));
        assert_eq!(answers.get("model"), Some("gpt-5"));
        assert!(String::from_utf8(output)?.contains("Configuration saved."));
        let settings: Value = serde_json::from_str(&fs::read_to_string(settings_path)?)?;
        assert_eq!(
            settings["agent"]["cli_onboarding"]["extension_path"].as_str(),
            Some(extension_path.to_string_lossy().as_ref())
        );
        Ok(())
    }
}
