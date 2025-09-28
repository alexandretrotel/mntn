mod cli;
mod logger;
mod tasks;
mod utils;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};
use tasks::{backup, biometric_sudo, clean, delete, install, link, purge, restore};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Backup) => backup::run(),
        Some(Commands::Clean(args)) => clean::run(args),
        Some(Commands::Purge) => purge::run(),
        Some(Commands::Link) => link::run(),
        Some(Commands::Delete(args)) => delete::run(args),
        Some(Commands::Install(args)) => install::run(args),
        Some(Commands::BiometricSudo) => biometric_sudo::run(),
        Some(Commands::Restore) => restore::run(),
        None => {
            Cli::command().print_help().expect("Failed to print help");
        }
    }
}
