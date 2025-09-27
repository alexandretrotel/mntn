use clap::{Parser, Subcommand};

/// Command line interface for `mntn`.
/// ```
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

/// Supported subcommands for the `mntn` CLI.
#[derive(Subcommand)]
pub enum Commands {
    Backup,
    BiometricSudo,
    Clean,
    Delete,
    Install,
    Link,
    Purge,
    Restore,
}
