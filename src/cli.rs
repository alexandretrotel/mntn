use clap::{Args, Parser, Subcommand};

/// Command line interface for `mntn`.
#[derive(Parser)]
#[command(
    name = "mntn",
    version = env!("CARGO_PKG_VERSION"),
    about = "A Rust-based CLI tool for system maintenance."
)]
pub struct Cli {
    /// Subcommand to run
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Arguments for commands that require additional options.
#[derive(Args)]
pub struct CleanArgs {
    /// Also clean system files (requires sudo)
    #[arg(long, short = 's')]
    pub system: bool,
    /// Show what would be cleaned without actually cleaning
    #[arg(long, short = 'n')]
    pub dry_run: bool,
}

#[derive(Args)]
pub struct DeleteArgs {
    /// Permanently delete files instead of moving to trash
    #[arg(long, short = 'p')]
    pub permanent: bool,
    /// Show what would be deleted without actually deleting
    #[arg(long, short = 'n')]
    pub dry_run: bool,
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
    #[cfg(target_os = "macos")]
    Delete(DeleteArgs),
    Install,
    Link,
    #[cfg(target_os = "macos")]
    Purge,
    Restore,
}
