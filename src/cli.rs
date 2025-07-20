use clap::{Parser, Subcommand};

/// Command line interface for `mntn` - a Rust-based macOS maintenance tool.
///
/// Supports various maintenance-related subcommands like backup, cleaning, and linking dotfiles.
///
/// # Examples
///
/// ```
/// use clap::Parser;
///
/// let cli = Cli::parse();
/// match &cli.command {
///     Some(Commands::Backup) => { /* perform backup */ }
///     Some(Commands::Clean) => { /* perform cleaning */ }
///     _ => {}
/// }
/// ```
#[derive(Parser)]
#[command(
    name = "mntn",
    version = env!("CARGO_PKG_VERSION"),
    about = "Rust-based macOS maintenance CLI"
)]
pub struct Cli {
    /// Subcommand to run
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Supported subcommands for the `mntn` CLI.
///
/// Each variant corresponds to a maintenance task.
///
/// # Variants
///
/// * `Backup` - Backup global packages like brew, npm, uv, cargo, bun.
/// * `Clean` - Clean system junk safely.
/// * `Purge` - Purge unused launch agents and daemons.
/// * `Install` - Install launch agents and perform backup + clean.
/// * `Link` - Create symlinks for dotfiles.
/// * `Delete` - Remove an app bundle and its related files.
/// * `BiometricSudo` - Use Touch ID to enable sudo in your terminal.
/// * `Restore` - Restore everything from your backup.
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
