mod cli;
mod logger;
mod tasks;
mod utils;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};
use tasks::{backup, biometric_sudo, clean, delete, install, link, purge, restore};

/// Entry point for the application.
///
/// Parses command-line arguments and dispatches to the corresponding task module's run function.
///
/// Supported commands are defined in the `Commands` enum.
///
/// If no command is provided, prints the CLI help message.
///
/// # Panics
/// Panics if printing the help message fails (which should be very rare).
///
/// # Examples
/// ```no_run
/// // Run the CLI application:
/// fn main() {
///     // ...
/// }
/// ```
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
        Some(Commands::Restore) => restore::run(),
        None => {
            Cli::command().print_help().expect("Failed to print help");
        }
    }
}
