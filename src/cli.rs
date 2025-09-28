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

/// Arguments for the clean command.
#[derive(Args)]
pub struct CleanArgs {
    /// Also clean system files like caches, logs, and temporary files (requires sudo)
    #[arg(
        long,
        short = 's',
        help = "Clean system-wide files in addition to user files"
    )]
    pub system: bool,
    /// Preview what would be cleaned without actually removing any files
    #[arg(
        long,
        short = 'n',
        help = "Show what would be cleaned without performing any actions"
    )]
    pub dry_run: bool,
}

/// Arguments for the delete command.
#[derive(Args)]
pub struct DeleteArgs {
    /// Permanently delete files instead of moving them to the trash
    #[arg(long, short = 'p', help = "Bypass trash and permanently delete files")]
    pub permanent: bool,
    /// Preview what would be deleted without actually removing any files
    #[arg(
        long,
        short = 'n',
        help = "Show what would be deleted without performing any actions"
    )]
    pub dry_run: bool,
}

/// Arguments for the install command.
#[derive(Args)]
pub struct InstallArgs {
    /// Additionally schedule a daily clean task to run automatically
    #[arg(
        long,
        help = "Set up automatic daily cleaning in addition to installing"
    )]
    pub with_clean: bool,
}

/// Arguments for the purge command.
#[derive(Args)]
pub struct PurgeArgs {
    /// Also purge system files and configurations (requires sudo)
    #[arg(
        long,
        short = 's',
        help = "Remove system-wide files and configurations"
    )]
    pub system: bool,
    /// Preview what would be purged without actually removing any files
    #[arg(
        long,
        short = 'n',
        help = "Show what would be purged without performing any actions"
    )]
    pub dry_run: bool,
}

/// Arguments for the registry command.
#[derive(Args)]
pub struct RegistryArgs {
    #[command(subcommand)]
    pub action: RegistryActions,
}

/// Registry management actions.
#[derive(Subcommand)]
pub enum RegistryActions {
    /// List all registry entries
    #[command(about = "List all entries in the registry")]
    List {
        /// Filter by category
        #[arg(long, short = 'c', help = "Filter entries by category")]
        category: Option<String>,
        /// Show only enabled entries
        #[arg(long, short = 'e', help = "Show only enabled entries")]
        enabled_only: bool,
    },
    /// Add a new entry to the registry
    #[command(about = "Add a new entry to the registry")]
    Add {
        /// Unique ID for the entry
        #[arg(help = "Unique identifier for the registry entry")]
        id: String,
        /// Human-readable name
        #[arg(long, short = 'n', help = "Human-readable name for the entry")]
        name: String,
        /// Source path within backup directory
        #[arg(long, short = 's', help = "Source path within ~/.mntn/backup/")]
        source: String,
        /// Target path where file should be linked
        #[arg(long, short = 't', help = "Target path where file should be linked")]
        target: String,
        /// Category for organization
        #[arg(long, short = 'c', help = "Category for organization")]
        category: String,
        /// Optional description
        #[arg(long, short = 'd', help = "Optional description")]
        description: Option<String>,
    },
    /// Remove an entry from the registry
    #[command(about = "Remove an entry from the registry")]
    Remove {
        /// ID of the entry to remove
        #[arg(help = "ID of the entry to remove")]
        id: String,
    },
    /// Enable or disable an entry
    #[command(about = "Enable or disable a registry entry")]
    Toggle {
        /// ID of the entry to toggle
        #[arg(help = "ID of the entry to toggle")]
        id: String,
        /// Enable the entry
        #[arg(long, short = 'e', help = "Enable the entry")]
        enable: bool,
    },
}

/// Available maintenance commands for `mntn`.
///
/// Some commands are only available on macOS systems.
#[derive(Subcommand)]
pub enum Commands {
    /// Create a backup of important system configurations and user data
    #[command(about = "Backup system configurations and user data to a safe location")]
    Backup,

    /// Configure biometric authentication for sudo operations (macOS only)
    #[cfg(target_os = "macos")]
    #[command(about = "Enable Touch ID or Face ID authentication for sudo commands")]
    BiometricSudo,

    /// Clean temporary files, caches, and unnecessary data from the system
    #[command(about = "Remove temporary files, caches, logs, and other unnecessary data")]
    Clean(CleanArgs),

    /// Delete specific files or directories with various deletion options (macOS only)
    #[cfg(target_os = "macos")]
    #[command(about = "Delete files and directories with options for permanent deletion")]
    Delete(DeleteArgs),

    /// Install and configure the mntn tool on your system
    #[command(about = "Install mntn and optionally set up automated maintenance tasks")]
    Install(InstallArgs),

    /// Create symbolic links for configurations and dotfiles
    #[command(about = "Create and manage symbolic links for dotfiles and configurations")]
    Link,

    /// Thoroughly remove files and reset configurations to defaults
    #[command(about = "Completely remove files and reset system configurations")]
    Purge(PurgeArgs),

    /// Restore system configurations and data from a previous backup
    #[command(about = "Restore system state from a previously created backup")]
    Restore,

    /// Manage the registry of files and folders to backup and link
    #[command(about = "Manage the registry of files and folders for backup and linking")]
    Registry(RegistryArgs),
}
