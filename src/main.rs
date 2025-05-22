mod cli;
mod logger;
mod tasks;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};
use tasks::{backup, clean, install, purge};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Backup) => backup::run(),
        Some(Commands::Clean) => clean::run(),
        Some(Commands::Purge) => purge::run(),
        Some(Commands::Install) | None => install::run(),
    }
}
