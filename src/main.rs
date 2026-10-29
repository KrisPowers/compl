//! Inline, as-you-type suggestions for commands and paths.
//!
//! `autocomplete run` starts your shell inside a pseudo terminal, passes every
//! byte through, and draws suggestions over the prompt while you type.

mod complete;
mod markers;
mod overlay;
mod screen;
mod session;
mod shell;
mod terminal;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use shell::Shell;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start a shell with suggestions turned on. This is the default.
    Run {
        /// The shell to start. Detected when left out.
        #[arg(long)]
        shell: Option<Shell>,
        /// Path to the shell's executable.
        #[arg(long)]
        exe: Option<PathBuf>,
        /// Start the shell as a login shell.
        #[arg(long)]
        login: bool,
    },
    /// Print the snippet a shell's startup file evaluates to start autocomplete.
    Init { shell: Shell },
    /// Add autocomplete to a shell's startup file, or take it out again.
    Setup {
        /// The shell to set up. Detected when left out.
        shell: Option<Shell>,
        /// Remove autocomplete from the startup file instead.
        #[arg(long)]
        remove: bool,
    },
    /// Print the suggestions for a command line.
    Complete {
        /// The shell whose quoting rules apply. Detected when left out.
        #[arg(long)]
        shell: Option<Shell>,
        /// The directory to complete paths in. Defaults to the current one.
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// The command line, up to the cursor.
        line: String,
    },
}

fn main() -> ExitCode {
    match dispatch(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("autocomplete: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        None => session::run(Shell::detect(), None, false),
        Some(Command::Run { shell, exe, login }) => {
            session::run(shell.unwrap_or_else(Shell::detect), exe, login)
        }
        Some(Command::Init { shell }) => {
            print!("{}", shell::init(shell)?);
            Ok(())
        }
        Some(Command::Setup { shell, remove }) => {
            shell::setup(shell.unwrap_or_else(Shell::detect), remove)
        }
        Some(Command::Complete { shell, cwd, line }) => {
            let cwd = match cwd {
                Some(cwd) => cwd,
                None => std::env::current_dir()?,
            };
            let mut ctx = complete::Context::new(shell.unwrap_or_else(Shell::detect), cwd);
            for item in complete::complete(&line, &mut ctx).items {
                match item.description {
                    Some(description) => println!("{}\t{description}", item.text),
                    None => println!("{}", item.text),
                }
            }
            Ok(())
        }
    }
}
