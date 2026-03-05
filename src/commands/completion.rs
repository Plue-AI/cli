use anyhow::Result;
use clap::{Args, Subcommand};
use clap_complete::Shell;
use std::io::{self, Write};

#[derive(Args)]
pub struct CompletionArgs {
    #[command(subcommand)]
    command: CompletionCommand,
}

#[derive(Subcommand)]
enum CompletionCommand {
    /// Generate bash completions
    Bash,
    /// Generate zsh completions
    Zsh,
    /// Generate fish completions
    Fish,
}

pub fn run(args: CompletionArgs) -> Result<()> {
    let shell = match args.command {
        CompletionCommand::Bash => Shell::Bash,
        CompletionCommand::Zsh => Shell::Zsh,
        CompletionCommand::Fish => Shell::Fish,
    };

    let mut cmd = <crate::Cli as clap::CommandFactory>::command();
    let mut buffer: Vec<u8> = Vec::new();
    clap_complete::generate(shell, &mut cmd, "plue", &mut buffer);

    let mut stdout = io::stdout();
    if let Err(err) = stdout.write_all(&buffer) {
        if err.kind() == io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(err.into());
    }
    if let Err(err) = stdout.flush() {
        if err.kind() == io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(err.into());
    }
    Ok(())
}
