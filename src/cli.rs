use clap::{Args, Parser, Subcommand};

/// Command line interface for `mntn`.
#[derive(Parser)]
#[command(
    name = "mntn",
    version = env!("CARGO_PKG_VERSION"),
    about = "A Rust-based CLI tool for macOS system maintenance."
)]
pub struct Cli {
    /// Subcommand to run
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Arguments for commands that require additional options.
#[derive(Args)]
pub struct CleanArgs {
    #[arg(long, short)]
    pub system: bool,
}

/// Supported subcommands for `mntn`.
///
/// Some commands are only available on macOS.
#[derive(Subcommand)]
pub enum Commands {
    Backup,
    #[cfg(target_os = "macos")]
    BiometricSudo,
    Clean(CleanArgs),
    Delete,
    Install,
    Link,
    #[cfg(target_os = "macos")]
    Purge,
    Restore,
}
