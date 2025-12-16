mod cli;
mod logger;
mod registries;
mod registry;
mod tasks;
mod utils;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};
use tasks::{
    backup, biometric_sudo, clean, configs_registry as configs_registry_task, delete, install,
    link, package_registry as package_registry_task, purge, restore, sync, validate,
};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Backup) => backup::run(),
        Some(Commands::Clean(args)) => clean::run(args),
        Some(Commands::Purge(args)) => purge::run(args),
        Some(Commands::Link) => link::run(),
        Some(Commands::Delete(args)) => delete::run(args),
        Some(Commands::Install(args)) => install::run(args),
        Some(Commands::BiometricSudo) => biometric_sudo::run(),
        Some(Commands::Restore) => restore::run(),
        Some(Commands::Registry(args)) => configs_registry_task::run(args),
        Some(Commands::PackageRegistry(args)) => package_registry_task::run(args),
        Some(Commands::Sync(args)) => sync::run(args),
        Some(Commands::Validate) => validate::run(),
        None => {
            Cli::command().print_help().expect("Failed to print help");
        }
    }
}
