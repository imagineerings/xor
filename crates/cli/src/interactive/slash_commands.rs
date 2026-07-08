use super::session::InteractiveSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashCommandKind {
    TuiLocal,
    AgentForwarded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub kind: SlashCommandKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandSuggestion {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub kind: SlashCommandKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSlashCommand {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandOutcome {
    Help(String),
    ConversationCleared,
    SaveRequested(Option<String>),
    LoadRequested(Option<String>),
    ModelRequested(Option<String>),
    ForwardToAgent {
        command: String,
        arguments: String,
    },
    Unknown {
        command: String,
        suggestions: Vec<SlashCommandSuggestion>,
    },
    NotCommand,
}

#[derive(Debug, Clone)]
pub struct SlashCommandCatalog {
    commands: Vec<SlashCommand>,
}

impl SlashCommandCatalog {
    pub fn new(commands: Vec<SlashCommand>) -> Self {
        Self { commands }
    }

    pub fn tui_default() -> Self {
        Self::new(vec![
            tui_command("help", "Show available slash commands", "/help"),
            tui_command("clear", "Clear the current terminal conversation", "/clear"),
            tui_command("save", "Request saving the current session", "/save [path]"),
            tui_command("load", "Request loading a saved session", "/load [path]"),
            tui_command(
                "model",
                "Show or change the active model",
                "/model [model-id]",
            ),
            agent_command(
                "recipe",
                "List or run recipes through shared agent command handling",
                "/recipe [name]",
            ),
            agent_command(
                "skill",
                "Invoke a skill through shared agent command handling",
                "/skill <name>",
            ),
            agent_command(
                "compact",
                "Compact the conversation through shared agent command handling",
                "/compact",
            ),
        ])
    }

    pub fn commands(&self) -> &[SlashCommand] {
        &self.commands
    }

    pub fn find(&self, name: &str) -> Option<&SlashCommand> {
        self.commands.iter().find(|command| command.name == name)
    }

    pub fn suggestions(&self, query: &str) -> Vec<SlashCommandSuggestion> {
        let query = query.trim_start_matches('/').to_ascii_lowercase();
        let mut suggestions = self
            .commands
            .iter()
            .filter(|command| {
                query.is_empty() || command.name.to_ascii_lowercase().starts_with(&query)
            })
            .map(|command| SlashCommandSuggestion {
                name: command.name.clone(),
                description: command.description.clone(),
                usage: command.usage.clone(),
                kind: command.kind,
            })
            .collect::<Vec<_>>();

        suggestions.sort_by(|left, right| {
            command_kind_order(left.kind)
                .cmp(&command_kind_order(right.kind))
                .then_with(|| left.name.cmp(&right.name))
        });
        suggestions
    }

    pub fn help_text(&self) -> String {
        let mut help = String::from("## Available Commands\n\n");
        for command in &self.commands {
            let source = match command.kind {
                SlashCommandKind::TuiLocal => "TUI",
                SlashCommandKind::AgentForwarded => "Agent",
            };
            help.push_str(&format!(
                "- **`/{}`** ({source}) - {}\n  `{}`\n",
                command.name, command.description, command.usage
            ));
        }
        help
    }
}

#[derive(Debug, Clone, Default)]
pub struct SlashCommandParser;

impl SlashCommandParser {
    pub fn parse(input: &str) -> Option<ParsedSlashCommand> {
        let trimmed = input.trim_start();
        let rest = trimmed.strip_prefix('/')?;
        if rest
            .chars()
            .next()
            .is_none_or(|character| character.is_whitespace())
        {
            return None;
        }

        let name_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let name = &rest[..name_end];
        if name.is_empty() {
            return None;
        }
        let arguments = rest
            .get(name_end..)
            .map(str::trim_start)
            .unwrap_or_default()
            .to_string();
        Some(ParsedSlashCommand {
            name: name.to_string(),
            arguments,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SlashCommandRouter {
    catalog: SlashCommandCatalog,
}

impl SlashCommandRouter {
    pub fn new(catalog: SlashCommandCatalog) -> Self {
        Self { catalog }
    }

    pub fn tui_default() -> Self {
        Self::new(SlashCommandCatalog::tui_default())
    }

    pub fn catalog(&self) -> &SlashCommandCatalog {
        &self.catalog
    }

    pub fn suggestions(&self, query: &str) -> Vec<SlashCommandSuggestion> {
        self.catalog.suggestions(query)
    }

    pub fn handle(&self, input: &str, session: &mut InteractiveSession) -> SlashCommandOutcome {
        let Some(parsed) = SlashCommandParser::parse(input) else {
            return SlashCommandOutcome::NotCommand;
        };

        let Some(command) = self.catalog.find(&parsed.name) else {
            return SlashCommandOutcome::Unknown {
                command: parsed.name,
                suggestions: self.catalog.suggestions(input),
            };
        };

        match (command.kind, command.name.as_str()) {
            (SlashCommandKind::TuiLocal, "help") => {
                SlashCommandOutcome::Help(self.catalog.help_text())
            }
            (SlashCommandKind::TuiLocal, "clear") => {
                session.clear_conversation();
                SlashCommandOutcome::ConversationCleared
            }
            (SlashCommandKind::TuiLocal, "save") => {
                SlashCommandOutcome::SaveRequested(optional_argument(parsed.arguments))
            }
            (SlashCommandKind::TuiLocal, "load") => {
                SlashCommandOutcome::LoadRequested(optional_argument(parsed.arguments))
            }
            (SlashCommandKind::TuiLocal, "model") => {
                SlashCommandOutcome::ModelRequested(optional_argument(parsed.arguments))
            }
            (SlashCommandKind::AgentForwarded, _) => SlashCommandOutcome::ForwardToAgent {
                command: command.name.clone(),
                arguments: parsed.arguments,
            },
            (SlashCommandKind::TuiLocal, _) => SlashCommandOutcome::Unknown {
                command: parsed.name,
                suggestions: self.catalog.suggestions(input),
            },
        }
    }
}

fn tui_command(name: &str, description: &str, usage: &str) -> SlashCommand {
    SlashCommand {
        name: name.to_string(),
        description: description.to_string(),
        usage: usage.to_string(),
        kind: SlashCommandKind::TuiLocal,
    }
}

fn agent_command(name: &str, description: &str, usage: &str) -> SlashCommand {
    SlashCommand {
        name: name.to_string(),
        description: description.to_string(),
        usage: usage.to_string(),
        kind: SlashCommandKind::AgentForwarded,
    }
}

fn optional_argument(arguments: String) -> Option<String> {
    if arguments.trim().is_empty() {
        None
    } else {
        Some(arguments)
    }
}

fn command_kind_order(kind: SlashCommandKind) -> u8 {
    match kind {
        SlashCommandKind::TuiLocal => 0,
        SlashCommandKind::AgentForwarded => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_leading_slash_command_with_arguments() {
        assert_eq!(
            SlashCommandParser::parse("  /recipe release plan"),
            Some(ParsedSlashCommand {
                name: "recipe".to_string(),
                arguments: "release plan".to_string(),
            })
        );
        assert_eq!(SlashCommandParser::parse("not /help"), None);
        assert_eq!(SlashCommandParser::parse("/ help"), None);
    }

    #[test]
    fn suggests_commands_from_catalog() {
        let catalog = SlashCommandCatalog::tui_default();
        let suggestions = catalog.suggestions("/re");

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].name, "recipe");
        assert_eq!(suggestions[0].kind, SlashCommandKind::AgentForwarded);
    }

    #[test]
    fn handles_tui_only_commands() {
        let router = SlashCommandRouter::tui_default();
        let mut session = InteractiveSession::new();
        session.submit_user_message("hello".to_string());

        assert!(session.is_awaiting_agent_output());
        assert_eq!(
            router.handle("/clear", &mut session),
            SlashCommandOutcome::ConversationCleared
        );
        assert!(session.conversation().is_empty());
        assert!(!session.is_awaiting_agent_output());

        assert_eq!(
            router.handle("/model gpt-5", &mut session),
            SlashCommandOutcome::ModelRequested(Some("gpt-5".to_string()))
        );
    }

    #[test]
    fn forwards_shared_agent_commands() {
        let router = SlashCommandRouter::tui_default();
        let mut session = InteractiveSession::new();

        assert_eq!(
            router.handle("/compact summarize setup", &mut session),
            SlashCommandOutcome::ForwardToAgent {
                command: "compact".to_string(),
                arguments: "summarize setup".to_string(),
            }
        );
        assert_eq!(
            router.handle("/skill review", &mut session),
            SlashCommandOutcome::ForwardToAgent {
                command: "skill".to_string(),
                arguments: "review".to_string(),
            }
        );
    }

    #[test]
    fn renders_help_from_catalog() {
        let router = SlashCommandRouter::tui_default();
        let mut session = InteractiveSession::new();
        let SlashCommandOutcome::Help(help) = router.handle("/help", &mut session) else {
            panic!("expected help outcome");
        };

        assert!(help.contains("/help"));
        assert!(help.contains("/recipe"));
        assert!(help.contains("Agent"));
    }
}
