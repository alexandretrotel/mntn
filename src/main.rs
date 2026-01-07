mod cli;
mod encryption;
mod logger;
mod profile;
mod registries;
mod registry;
mod tasks;
mod utils;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};
use tasks::{
    backup, clean, configs_registry as configs_registry_task,
    encrypted_configs_registry as encrypted_configs_registry_task, install, migrate,
    package_registry as package_registry_task, profile as profile_task, purge, restore, setup,
    sync, use_profile, validate,
};

#[cfg(target_os = "macos")]
use tasks::{biometric_sudo, delete};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Backup(args)) => backup::run_with_args(args),
        Some(Commands::Clean(args)) => clean::run_with_args(args),
        Some(Commands::Purge(args)) => purge::run_with_args(args),
        Some(Commands::Install(args)) => install::run_with_args(args),
        Some(Commands::Restore(args)) => restore::run_with_args(args),
        Some(Commands::RegistryConfigs(args)) => configs_registry_task::run_with_args(args),
        Some(Commands::RegistryPackages(args)) => package_registry_task::run_with_args(args),
        Some(Commands::RegistryEncrypted(args)) => {
            encrypted_configs_registry_task::run_with_args(args)
        }
        Some(Commands::Use(args)) => use_profile::run_with_args(args),
        Some(Commands::Profile(args)) => profile_task::run_with_args(args),
        Some(Commands::Sync(args)) => sync::run_with_args(args),
        Some(Commands::Validate(args)) => validate::run_with_args(args),
        Some(Commands::Migrate(args)) => migrate::run_with_args(args),
        Some(Commands::Setup) => setup::run(),
        #[cfg(target_os = "macos")]
        Some(Commands::Delete(args)) => delete::run_with_args(args),
        #[cfg(target_os = "macos")]
        Some(Commands::BiometricSudo(args)) => biometric_sudo::run_with_args(args),
        None => {
            Cli::command().print_help().expect("Failed to print help");
        }
    }
}
