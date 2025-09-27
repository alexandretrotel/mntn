use clap::{Parser, Subcommand};

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

/// Supported subcommands for `mntn`.
///
/// Some commands are only available on macOS.
#[derive(Subcommand)]
pub enum Commands {
    Backup,
    #[cfg(target_os = "macos")]
    BiometricSudo,
    Clean,
    Delete,
    Install,
    Link,
    #[cfg(target_os = "macos")]
    Purge,
    Restore,
}
