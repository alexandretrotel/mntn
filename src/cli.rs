use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "mntn",
    version = env!("CARGO_PKG_VERSION"),
    about = "Rust-based macOS maintenance CLI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Backup global packages (e.g. brew, npm, uv, cargo, bun)
    Backup,
    /// Clean system junk safely
    Clean,
    /// Purge unused launch agents/daemons
    Purge,
    /// Install launch agents and perform backup+clean
    Install,
    /// Create symlinks for dotfiles
    Link,
    /// Remove an app bundle and its related files
    Delete,
    /// Use Touch ID to enable sudo in your terminal
    BiometricSudo,
    /// Restore everything from your backup
    Restore,
}
