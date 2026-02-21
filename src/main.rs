mod cli;
mod commands;
mod encryption;
mod errors;
mod profiles;
mod registry;
mod utils;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};
use commands::{backup, git, profile, restore, r#use, validate};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Backup(args)) => backup::run(args),
        Some(Commands::Restore(args)) => restore::run(args),
        Some(Commands::Use(args)) => r#use::run(args),
        Some(Commands::Profile(args)) => profile::run(args),
        Some(Commands::Git(args)) => git::run(args),
        Some(Commands::Validate(args)) => validate::run(args),
        None => {
            Cli::command().print_help().expect("Failed to print help");
        }
    }
}
