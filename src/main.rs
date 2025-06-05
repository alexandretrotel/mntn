mod cli;
mod logger;
mod tasks;
mod utils;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};
use tasks::{backup, biometric_sudo, clean, delete, install, link, purge};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Backup) => backup::run(),
        Some(Commands::Clean) => clean::run(),
        Some(Commands::Purge) => purge::run(),
        Some(Commands::Link) => link::run(),
        Some(Commands::Delete) => delete::run(),
        Some(Commands::Install) => install::run(),
        Some(Commands::BiometricSudo) => biometric_sudo::run(),
        None => {
            Cli::command().print_help().expect("Failed to print help");
        }
    }
}
