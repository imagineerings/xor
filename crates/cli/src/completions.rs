mod shells {
    pub use clap_complete::aot::{Bash, Elvish, Fish, PowerShell, Zsh};
    pub use clap_complete_nushell::Nushell;
}

use clap_complete::Generator;

#[derive(Clone, Debug, clap::ValueEnum)]
#[non_exhaustive]
#[value(rename_all = "lower")]
pub(crate) enum Shell {
    Bash,
    Elvish,
    Fish,
    Nushell,
    PowerShell,
    Zsh,
}

impl Generator for Shell {
    fn file_name(&self, name: &str) -> String {
        match self {
            Self::Bash => shells::Bash.file_name(name),
            Self::Elvish => shells::Elvish.file_name(name),
            Self::Fish => shells::Fish.file_name(name),
            Self::Nushell => shells::Nushell.file_name(name),
            Self::PowerShell => shells::PowerShell.file_name(name),
            Self::Zsh => shells::Zsh.file_name(name),
        }
    }

    fn generate(&self, command: &clap::Command, buffer: &mut dyn std::io::Write) {
        match self {
            Self::Bash => shells::Bash.generate(command, buffer),
            Self::Elvish => shells::Elvish.generate(command, buffer),
            Self::Fish => shells::Fish.generate(command, buffer),
            Self::Nushell => shells::Nushell.generate(command, buffer),
            Self::PowerShell => shells::PowerShell.generate(command, buffer),
            Self::Zsh => shells::Zsh.generate(command, buffer),
        }
    }
}

pub(crate) fn main(command: &clap::Command, shell: &Shell) {
    let buffer = &mut std::io::stdout();
    shell.generate(command, buffer);
}
