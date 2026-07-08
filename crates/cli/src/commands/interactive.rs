use std::io::{self, BufRead, ErrorKind, Write};

use anyhow::Result;
use clap::Parser;
use cli::interactive::{
    InputEvent, InteractiveSession, SlashCommandOutcome, SlashCommandRouter, TerminalDimensions,
    TerminalRenderer,
};

use super::onboarding::{CliOnboarding, OnboardingStateStore};

#[derive(Parser, Debug)]
#[command(
    name = "interactive",
    about = "Start an interactive Sim terminal session"
)]
struct InteractiveArgs {
    #[arg(long)]
    no_onboarding: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractiveLoopEvent {
    Submitted(String),
    Slash(SlashCommandOutcome),
    Shutdown,
}

pub struct InteractiveCli {
    session: InteractiveSession,
    renderer: TerminalRenderer,
    slash_commands: SlashCommandRouter,
}

impl InteractiveCli {
    pub fn new(dimensions: TerminalDimensions) -> Self {
        Self {
            session: InteractiveSession::new(),
            renderer: TerminalRenderer::new(dimensions),
            slash_commands: SlashCommandRouter::tui_default(),
        }
    }

    pub fn session(&self) -> &InteractiveSession {
        &self.session
    }

    pub fn handle_line(&mut self, line: &str) -> InteractiveLoopEvent {
        let trimmed = line.trim_end();
        if trimmed == "/quit" || trimmed == "/exit" {
            return InteractiveLoopEvent::Shutdown;
        }

        if trimmed.trim_start().starts_with('/') {
            let outcome = self.slash_commands.handle(trimmed, &mut self.session);
            return InteractiveLoopEvent::Slash(outcome);
        }

        self.session.input_mut().set_buffer(trimmed);
        let event = self.session.handle_input_event(InputEvent::Submit);
        match event {
            cli::interactive::SessionEvent::UserMessageSubmitted(message) => {
                InteractiveLoopEvent::Submitted(message)
            }
            _ => InteractiveLoopEvent::Submitted(String::new()),
        }
    }

    pub fn render_outcome(&mut self, event: InteractiveLoopEvent) -> Vec<String> {
        match event {
            InteractiveLoopEvent::Submitted(message) if message.is_empty() => Vec::new(),
            InteractiveLoopEvent::Submitted(message) => {
                self.session.receive_agent_output(format!(
                    "Interactive agent runtime is not connected yet. Received: `{message}`"
                ));
                self.session
                    .conversation()
                    .last()
                    .map(|message| self.renderer.render_message(message))
                    .unwrap_or_default()
            }
            InteractiveLoopEvent::Slash(SlashCommandOutcome::Help(help)) => {
                self.renderer.render_markdown(&help)
            }
            InteractiveLoopEvent::Slash(SlashCommandOutcome::ConversationCleared) => {
                vec!["Conversation cleared.".to_string()]
            }
            InteractiveLoopEvent::Slash(SlashCommandOutcome::SaveRequested(path)) => {
                vec![format!(
                    "Save requested{}.",
                    path.map(|path| format!(" for {path}")).unwrap_or_default()
                )]
            }
            InteractiveLoopEvent::Slash(SlashCommandOutcome::LoadRequested(path)) => {
                vec![format!(
                    "Load requested{}.",
                    path.map(|path| format!(" for {path}")).unwrap_or_default()
                )]
            }
            InteractiveLoopEvent::Slash(SlashCommandOutcome::ModelRequested(model)) => {
                vec![
                    model
                        .map(|model| format!("Model change requested: {model}"))
                        .unwrap_or_else(|| "Model status requested.".to_string()),
                ]
            }
            InteractiveLoopEvent::Slash(SlashCommandOutcome::ForwardToAgent {
                command,
                arguments,
            }) => vec![format!(
                "Forwarded /{command}{} to shared agent handling.",
                if arguments.is_empty() {
                    String::new()
                } else {
                    format!(" {arguments}")
                }
            )],
            InteractiveLoopEvent::Slash(SlashCommandOutcome::Unknown {
                command,
                suggestions,
            }) => {
                let suggestion_text = suggestions
                    .into_iter()
                    .map(|suggestion| format!("/{}", suggestion.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                vec![if suggestion_text.is_empty() {
                    format!("Unknown command: /{command}")
                } else {
                    format!("Unknown command: /{command}. Suggestions: {suggestion_text}")
                }]
            }
            InteractiveLoopEvent::Slash(SlashCommandOutcome::NotCommand) => Vec::new(),
            InteractiveLoopEvent::Shutdown => vec!["Goodbye.".to_string()],
        }
    }
}

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let args = InteractiveArgs::try_parse_from(args)?;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout();
    run_interactive_with_io(&mut input, &mut output, args.no_onboarding)
}

pub fn run_interactive_with_io(
    input: &mut impl BufRead,
    output: &mut impl Write,
    no_onboarding: bool,
) -> Result<()> {
    if !no_onboarding {
        let onboarding_state = OnboardingStateStore::default_path();
        if CliOnboarding::is_first_run(&onboarding_state) {
            writeln!(
                output,
                "First run detected. Run `sim onboarding` for setup help."
            )?;
        }
    }

    let mut cli = InteractiveCli::new(TerminalDimensions::default());
    writeln!(
        output,
        "Sim interactive. Type /help for commands, /exit to quit."
    )?;

    loop {
        write!(output, "> ")?;
        output.flush()?;

        let mut line = String::new();
        let bytes = match input.read_line(&mut line) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::Interrupted => {
                writeln!(output)?;
                break;
            }
            Err(error) => return Err(error.into()),
        };
        if bytes == 0 {
            writeln!(output)?;
            break;
        }

        let event = cli.handle_line(&line);
        let should_shutdown = matches!(event, InteractiveLoopEvent::Shutdown);
        for rendered_line in cli.render_outcome(event) {
            writeln!(output, "{rendered_line}")?;
        }
        if should_shutdown {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn handles_user_input_and_shutdown() {
        let mut cli = InteractiveCli::new(TerminalDimensions::default());

        assert_eq!(
            cli.handle_line("hello\n"),
            InteractiveLoopEvent::Submitted("hello".to_string())
        );
        assert_eq!(cli.session().conversation().len(), 1);
        assert_eq!(cli.handle_line("/exit\n"), InteractiveLoopEvent::Shutdown);
    }

    #[test]
    fn handles_slash_commands() {
        let mut cli = InteractiveCli::new(TerminalDimensions::default());

        let InteractiveLoopEvent::Slash(SlashCommandOutcome::Help(help)) = cli.handle_line("/help")
        else {
            panic!("expected help outcome");
        };
        assert!(help.contains("/clear"));
    }

    #[test]
    fn runs_interactive_loop_until_exit() -> Result<()> {
        let mut input = Cursor::new("/help\nhello\n/exit\n");
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output, true)?;

        let output = String::from_utf8(output)?;
        assert!(output.contains("Sim interactive"));
        assert!(output.contains("Available Commands"));
        assert!(output.contains("Received:"));
        assert!(output.contains("Goodbye."));
        Ok(())
    }
}
