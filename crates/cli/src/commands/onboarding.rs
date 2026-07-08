use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "onboarding", about = "Run the Sim CLI onboarding flow")]
struct OnboardingArgs {
    #[arg(long, value_name = "FILE", hide = true)]
    state_file: Option<PathBuf>,
    #[arg(long)]
    reset: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingStep {
    Welcome,
    ProviderSetup,
    FirstMessage,
    ExtensionIntro,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingState {
    pub completed: bool,
    pub provider_setup_offered: bool,
    pub tutorial_offered: bool,
    pub extensions_introduced: bool,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            completed: false,
            provider_setup_offered: false,
            tutorial_offered: false,
            extensions_introduced: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CliOnboarding {
    steps: Vec<OnboardingStep>,
    current_step: usize,
    state: OnboardingState,
}

impl CliOnboarding {
    pub fn new(state: OnboardingState) -> Self {
        Self {
            steps: vec![
                OnboardingStep::Welcome,
                OnboardingStep::ProviderSetup,
                OnboardingStep::FirstMessage,
                OnboardingStep::ExtensionIntro,
                OnboardingStep::Complete,
            ],
            current_step: 0,
            state,
        }
    }

    pub fn is_first_run(state_file: &Path) -> bool {
        !state_file.exists()
            || OnboardingStateStore::new(state_file.to_path_buf())
                .load()
                .map(|state| !state.completed)
                .unwrap_or(true)
    }

    pub fn current_step(&self) -> Option<OnboardingStep> {
        self.steps.get(self.current_step).copied()
    }

    pub fn state(&self) -> &OnboardingState {
        &self.state
    }

    pub fn prompt(&self) -> Option<&'static str> {
        match self.current_step()? {
            OnboardingStep::Welcome => Some(
                "Welcome to Sim's terminal agent. This short setup gets your model, extensions, and first prompt ready.",
            ),
            OnboardingStep::ProviderSetup => {
                Some("Run provider setup now? You can also run `sim configure` later. [Y/n]: ")
            }
            OnboardingStep::FirstMessage => Some("Try a tutorial prompt after setup? [Y/n]: "),
            OnboardingStep::ExtensionIntro => Some(
                "Extensions can add tools and workflows. Run `sim extension add <path>` when you have one ready.",
            ),
            OnboardingStep::Complete => Some("Onboarding complete."),
        }
    }

    pub fn advance(&mut self, input: &str) -> Result<OnboardingEvent> {
        let Some(step) = self.current_step() else {
            self.state.completed = true;
            return Ok(OnboardingEvent::Completed);
        };

        let event = match step {
            OnboardingStep::Welcome => OnboardingEvent::MessageShown,
            OnboardingStep::ProviderSetup => {
                self.state.provider_setup_offered = true;
                if confirm_default_yes(input)? {
                    OnboardingEvent::RunConfigure
                } else {
                    OnboardingEvent::SkippedConfigure
                }
            }
            OnboardingStep::FirstMessage => {
                self.state.tutorial_offered = true;
                if confirm_default_yes(input)? {
                    OnboardingEvent::TutorialPrompt(
                        "Ask Sim to summarize this project and suggest the next small change."
                            .to_string(),
                    )
                } else {
                    OnboardingEvent::SkippedTutorial
                }
            }
            OnboardingStep::ExtensionIntro => {
                self.state.extensions_introduced = true;
                OnboardingEvent::MessageShown
            }
            OnboardingStep::Complete => {
                self.state.completed = true;
                OnboardingEvent::Completed
            }
        };

        self.current_step += 1;
        Ok(event)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnboardingEvent {
    MessageShown,
    RunConfigure,
    SkippedConfigure,
    TutorialPrompt(String),
    SkippedTutorial,
    Completed,
}

#[derive(Debug, Clone)]
pub struct OnboardingStateStore {
    path: PathBuf,
}

impl OnboardingStateStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> PathBuf {
        paths::config_dir().join("cli_onboarding.json")
    }

    pub fn load(&self) -> Result<OnboardingState> {
        if !self.path.exists() {
            return Ok(OnboardingState::default());
        }
        let content = fs::read_to_string(&self.path)
            .with_context(|| format!("reading onboarding state {}", self.path.display()))?;
        if content.trim().is_empty() {
            return Ok(OnboardingState::default());
        }
        serde_json::from_str(&content)
            .with_context(|| format!("parsing onboarding state {}", self.path.display()))
    }

    pub fn save(&self, state: &OnboardingState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("creating onboarding state directory {}", parent.display())
            })?;
        }
        let content = serde_json::to_string_pretty(state)?;
        fs::write(&self.path, format!("{content}\n"))
            .with_context(|| format!("writing onboarding state {}", self.path.display()))
    }

    pub fn reset(&self) -> Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error)
                .with_context(|| format!("removing onboarding state {}", self.path.display())),
        }
    }
}

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let args = OnboardingArgs::try_parse_from(args)?;
    let store = OnboardingStateStore::new(
        args.state_file
            .unwrap_or_else(OnboardingStateStore::default_path),
    );
    if args.reset {
        store.reset()?;
    }
    let state = store.load()?;
    let mut onboarding = CliOnboarding::new(state);
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout();
    run_onboarding_with_io(&mut onboarding, &mut input, &mut output, &store)
}

pub fn run_onboarding_with_io(
    onboarding: &mut CliOnboarding,
    input: &mut impl BufRead,
    output: &mut impl Write,
    store: &OnboardingStateStore,
) -> Result<()> {
    if onboarding.state().completed {
        writeln!(output, "CLI onboarding is already complete.")?;
        return Ok(());
    }

    while let Some(prompt) = onboarding.prompt() {
        let requires_input = matches!(
            onboarding.current_step(),
            Some(OnboardingStep::ProviderSetup) | Some(OnboardingStep::FirstMessage)
        );
        if requires_input {
            write!(output, "{prompt}")?;
            output.flush()?;
        } else {
            writeln!(output, "{prompt}")?;
        }

        let answer = if requires_input {
            let mut line = String::new();
            let bytes = input.read_line(&mut line)?;
            if bytes == 0 {
                return Err(anyhow!("onboarding input ended before completion"));
            }
            line
        } else {
            String::new()
        };

        let event = onboarding.advance(answer.trim_end())?;
        match event {
            OnboardingEvent::RunConfigure => {
                writeln!(output, "Run `sim configure` to complete provider setup.")?;
            }
            OnboardingEvent::TutorialPrompt(prompt) => {
                writeln!(output, "Tutorial prompt: {prompt}")?;
            }
            OnboardingEvent::Completed => {
                store.save(onboarding.state())?;
                break;
            }
            OnboardingEvent::MessageShown
            | OnboardingEvent::SkippedConfigure
            | OnboardingEvent::SkippedTutorial => {}
        }
    }

    store.save(onboarding.state())
}

fn confirm_default_yes(input: &str) -> Result<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "" | "y" | "yes" | "true" => Ok(true),
        "n" | "no" | "false" => Ok(false),
        value => Err(anyhow!(
            "invalid confirmation `{value}`; expected yes or no"
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn detects_first_run_from_missing_or_incomplete_state() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let state_path = temp_dir.path().join("onboarding.json");

        assert!(CliOnboarding::is_first_run(&state_path));
        OnboardingStateStore::new(state_path.clone()).save(&OnboardingState {
            completed: true,
            provider_setup_offered: true,
            tutorial_offered: true,
            extensions_introduced: true,
        })?;
        assert!(!CliOnboarding::is_first_run(&state_path));
        Ok(())
    }

    #[test]
    fn advances_through_provider_and_tutorial_prompts() -> Result<()> {
        let mut onboarding = CliOnboarding::new(OnboardingState::default());

        assert_eq!(onboarding.advance("")?, OnboardingEvent::MessageShown);
        assert_eq!(onboarding.advance("yes")?, OnboardingEvent::RunConfigure);
        assert!(matches!(
            onboarding.advance("")?,
            OnboardingEvent::TutorialPrompt(_)
        ));
        assert_eq!(onboarding.advance("")?, OnboardingEvent::MessageShown);
        assert_eq!(onboarding.advance("")?, OnboardingEvent::Completed);
        assert!(onboarding.state().completed);
        Ok(())
    }

    #[test]
    fn runs_onboarding_with_io_and_persists_completion() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let state_path = temp_dir.path().join("onboarding.json");
        let store = OnboardingStateStore::new(state_path.clone());
        let mut onboarding = CliOnboarding::new(OnboardingState::default());
        let mut input = Cursor::new("n\ny\n");
        let mut output = Vec::new();

        run_onboarding_with_io(&mut onboarding, &mut input, &mut output, &store)?;

        let state = store.load()?;
        assert!(state.completed);
        assert!(state.provider_setup_offered);
        assert!(state.tutorial_offered);
        assert!(state.extensions_introduced);
        let output = String::from_utf8(output)?;
        assert!(output.contains("Welcome"));
        assert!(output.contains("Tutorial prompt:"));
        Ok(())
    }
}
