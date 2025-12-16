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
        Some(Commands::Backup(args)) => backup::run_with_args(args),
        Some(Commands::Clean(args)) => clean::run_with_args(args),
        Some(Commands::Purge(args)) => purge::run_with_args(args),
        Some(Commands::Link(args)) => link::run_with_args(args),
        Some(Commands::Delete(args)) => delete::run(args),
        Some(Commands::Install(args)) => install::run_with_args(args),
        Some(Commands::BiometricSudo) => biometric_sudo::run(),
        Some(Commands::Restore(args)) => restore::run_with_args(args),
        Some(Commands::Registry(args)) => configs_registry_task::run(args),
        Some(Commands::PackageRegistry(args)) => package_registry_task::run(args),
        Some(Commands::Sync(args)) => sync::run_with_args(args),
        Some(Commands::Validate) => validate::run(),
        None => {
            Cli::command().print_help().expect("Failed to print help");
        }
    }
}
