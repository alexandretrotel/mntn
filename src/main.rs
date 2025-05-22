mod cli;
mod logger;
mod tasks;
mod utils;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};
use tasks::{backup, clean, install, link, purge};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Backup) => backup::run(),
        Some(Commands::Clean) => clean::run(),
        Some(Commands::Purge) => purge::run(),
        Some(Commands::Link) => link::run(),
        Some(Commands::Install) => install::run(),
        None => {
            Cli::command().print_help().expect("Failed to print help");
        }
    }
}
