use clap::{Args, Parser, Subcommand};

use crate::profile::ActiveProfile;

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

/// Arguments for the backup command.
#[derive(Args)]
pub struct BackupArgs {
    /// Preview what would be backed up without actually performing the backup
    #[arg(
        long,
        short = 'n',
        help = "Show what would be backed up without performing any actions"
    )]
    pub dry_run: bool,
    /// Target a specific profile for backup (defaults to active profile, or common if none)
    #[arg(long, short = 'p', help = "Target a specific profile for backup")]
    pub profile: Option<String>,
    /// Skip encrypted configs (by default, encrypted configs are included)
    #[arg(
        long,
        help = "Skip encrypted configs backup (will not prompt for password)"
    )]
    pub skip_encrypted: bool,
}

impl BackupArgs {
    pub fn resolve_profile(&self) -> ActiveProfile {
        ActiveProfile::resolve(self.profile.as_deref())
    }
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
    /// Preview what tasks would be installed without actually installing them
    #[arg(
        long,
        short = 'n',
        help = "Show what would be installed without performing any actions"
    )]
    pub dry_run: bool,
}

/// Arguments for the restore command.
#[derive(Args)]
pub struct RestoreArgs {
    /// Preview what would be restored without actually restoring
    #[arg(
        long,
        short = 'n',
        help = "Show what would be restored without performing any actions"
    )]
    pub dry_run: bool,
    /// Skip encrypted configs (by default, encrypted configs are included)
    #[arg(
        long,
        help = "Skip encrypted configs restore (will not prompt for password)"
    )]
    pub skip_encrypted: bool,
}

impl RestoreArgs {
    pub fn resolve_profile(&self) -> ActiveProfile {
        ActiveProfile::resolve(None)
    }
}

/// Arguments for the biometric sudo command.
#[derive(Args)]
pub struct BiometricSudoArgs {
    /// Preview what would be configured without actually performing the configuration
    #[arg(
        long,
        short = 'n',
        help = "Show what would be configured without performing any actions"
    )]
    pub dry_run: bool,
}

/// Arguments for the validate command.
#[derive(Args)]
pub struct ValidateArgs {
    /// Preview what would be validated without actually performing the validation
    #[arg(
        long,
        short = 'n',
        help = "Show what would be validated without performing any actions"
    )]
    pub dry_run: bool,
}

impl ValidateArgs {
    pub fn resolve_profile(&self) -> ActiveProfile {
        ActiveProfile::resolve(None)
    }
}

/// Arguments for the migrate command.
#[derive(Args)]
pub struct MigrateArgs {
    /// Preview what would be migrated without actually performing the migration
    #[arg(
        long,
        short = 'n',
        help = "Show what would be migrated without performing any actions"
    )]
    pub dry_run: bool,
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

/// Arguments for the sync command.
#[derive(Args)]
pub struct SyncArgs {
    /// Initialize a new git repository in ~/.mntn
    #[arg(
        long,
        help = "Initialize a new git repository in ~/.mntn with the provided remote URL"
    )]
    pub init: bool,
    /// Remote URL for git repository initialization
    #[arg(long, help = "Remote repository URL (required with --init)")]
    pub remote_url: Option<String>,
    /// Pull changes from remote repository
    #[arg(long, help = "Pull latest changes from remote repository")]
    pub pull: bool,
    /// Push changes to remote repository
    #[arg(long, help = "Push local changes to remote repository")]
    pub push: bool,
    /// Sync both ways (pull then push)
    #[arg(
        long,
        help = "Sync both ways: pull latest changes then push local changes"
    )]
    pub sync: bool,
    /// Custom commit message for push operations
    #[arg(
        long,
        short = 'm',
        help = "Custom commit message (default: timestamp-based message)"
    )]
    pub message: Option<String>,
    /// Automatically run 'mntn restore' after pulling changes
    #[arg(long, help = "Automatically run 'mntn restore' after pulling changes")]
    pub auto_restore: bool,
    /// Preview what would be synced without performing any actions
    #[arg(
        long,
        short = 'n',
        help = "Show what would be synced without performing any actions"
    )]
    pub dry_run: bool,
    /// Show git status for the repository
    #[arg(long, help = "Show git status for the repository")]
    pub status: bool,
}

/// Arguments for the registry command.
#[derive(Args)]
pub struct ConfigsRegistryArgs {
    #[command(subcommand)]
    pub action: ConfigsRegistryActions,
    /// Preview what would be changed without actually performing the changes
    #[arg(
        long,
        short = 'n',
        help = "Show what would be changed without performing any actions"
    )]
    pub dry_run: bool,
}

/// Registry management actions.
#[derive(Subcommand)]
pub enum ConfigsRegistryActions {
    /// List all registry entries
    #[command(about = "List all entries in the registry")]
    List {
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

/// Arguments for the package registry command.
#[derive(Args)]
pub struct PackageRegistryArgs {
    #[command(subcommand)]
    pub action: PackageRegistryActions,
    /// Preview what would be changed without actually performing the changes
    #[arg(
        long,
        short = 'n',
        help = "Show what would be changed without performing any actions"
    )]
    pub dry_run: bool,
}

/// Package registry management actions.
#[derive(Subcommand)]
pub enum PackageRegistryActions {
    /// List all package manager entries
    #[command(about = "List all package manager entries in the registry")]
    List {
        /// Show only enabled entries
        #[arg(long, short = 'e', help = "Show only enabled entries")]
        enabled_only: bool,
        /// Show only entries compatible with current platform
        #[arg(long, short = 'p', help = "Show only platform-compatible entries")]
        platform_only: bool,
    },
    /// Add a new package manager entry to the registry
    #[command(about = "Add a new package manager entry to the registry")]
    Add {
        /// Unique ID for the entry
        #[arg(help = "Unique identifier for the package manager entry")]
        id: String,
        /// Human-readable name
        #[arg(
            long,
            short = 'n',
            help = "Human-readable name for the package manager"
        )]
        name: String,
        /// Command to execute
        #[arg(long, short = 'c', help = "Command to execute (e.g., 'brew')")]
        command: String,
        /// Arguments for the command
        #[arg(
            long,
            short = 'a',
            help = "Arguments for the command (comma-separated)"
        )]
        args: String,
        /// Output filename
        #[arg(long, short = 'o', help = "Output filename (e.g., 'brew.txt')")]
        output_file: String,
        /// Optional description
        #[arg(long, short = 'd', help = "Optional description")]
        description: Option<String>,
        /// Platform compatibility (comma-separated)
        #[arg(
            long,
            short = 'p',
            help = "Platform compatibility (comma-separated, e.g., 'macos,linux')"
        )]
        platforms: Option<String>,
    },
    /// Remove a package manager entry from the registry
    #[command(about = "Remove a package manager entry from the registry")]
    Remove {
        /// ID of the entry to remove
        #[arg(help = "ID of the entry to remove")]
        id: String,
    },
    /// Enable or disable a package manager entry
    #[command(about = "Enable or disable a package manager entry")]
    Toggle {
        /// ID of the entry to toggle
        #[arg(help = "ID of the entry to toggle")]
        id: String,
        /// Enable the entry
        #[arg(long, short = 'e', help = "Enable the entry")]
        enable: bool,
    },
}

/// Arguments for the encrypted registry command.
#[derive(Args)]
pub struct EncryptedRegistryArgs {
    #[command(subcommand)]
    pub action: EncryptedRegistryActions,
    /// Preview what would be changed without actually performing the changes
    #[arg(
        long,
        short = 'n',
        help = "Show what would be changed without performing any actions"
    )]
    pub dry_run: bool,
}

/// Encrypted registry management actions.
#[derive(Subcommand)]
pub enum EncryptedRegistryActions {
    /// List all encrypted registry entries
    #[command(about = "List all entries in the encrypted registry")]
    List {
        /// Show only enabled entries
        #[arg(long, short = 'e', help = "Show only enabled entries")]
        enabled_only: bool,
    },
    /// Add a new entry to the encrypted registry
    #[command(about = "Add a new entry to the encrypted registry")]
    Add {
        /// Unique ID for the entry
        #[arg(help = "Unique identifier for the registry entry")]
        id: String,
        /// Human-readable name
        #[arg(long, short = 'n', help = "Human-readable name for the entry")]
        name: String,
        /// Source path within backup/encrypted directory
        #[arg(
            long,
            short = 's',
            help = "Source path within ~/.mntn/backup/*/encrypted/"
        )]
        source: String,
        /// Target path where file should be restored
        #[arg(long, short = 't', help = "Target path where file should be restored")]
        target: String,
        /// Optional description
        #[arg(long, short = 'd', help = "Optional description")]
        description: Option<String>,
        /// Encrypt the filename in backup
        #[arg(long, help = "Encrypt the filename when backing up")]
        encrypt_filename: bool,
    },
    /// Remove an entry from the encrypted registry
    #[command(about = "Remove an entry from the encrypted registry")]
    Remove {
        /// ID of the entry to remove
        #[arg(help = "ID of the entry to remove")]
        id: String,
    },
    /// Enable or disable an encrypted entry
    #[command(about = "Enable or disable an encrypted registry entry")]
    Toggle {
        /// ID of the entry to toggle
        #[arg(help = "ID of the entry to toggle")]
        id: String,
        /// Enable the entry
        #[arg(long, short = 'e', help = "Enable the entry")]
        enable: bool,
    },
}

/// Arguments for the use command.
#[derive(Args)]
pub struct UseArgs {
    /// Profile name to switch to (use 'common' or 'none' to clear active profile)
    #[arg(help = "Profile name to switch to")]
    pub profile: String,
    /// Preview what would be changed without actually switching
    #[arg(
        long,
        short = 'n',
        help = "Show what would be changed without performing any actions"
    )]
    pub dry_run: bool,
}

/// Arguments for the profile command.
#[derive(Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub action: Option<ProfileActions>,
}

/// Profile command actions.
#[derive(Subcommand)]
pub enum ProfileActions {
    /// List all available profiles
    #[command(about = "List all available profiles")]
    List,
    /// Create a new profile
    #[command(about = "Create a new profile")]
    Create {
        /// Name for the new profile
        #[arg(help = "Name for the new profile")]
        name: String,
        /// Optional description for the profile
        #[arg(long, short = 'd', help = "Optional description for the profile")]
        description: Option<String>,
    },
    /// Delete a profile
    #[command(about = "Delete a profile")]
    Delete {
        /// Name of the profile to delete
        #[arg(help = "Name of the profile to delete")]
        name: String,
    },
}

/// Available maintenance commands for `mntn`.
///
/// Some commands are only available on macOS systems.
#[derive(Subcommand)]
pub enum Commands {
    /// Create a backup of important system configurations and user data
    #[command(about = "Backup system configurations and user data to a safe location")]
    Backup(BackupArgs),

    /// Configure biometric authentication for sudo operations (macOS only)
    #[cfg(target_os = "macos")]
    #[command(about = "Enable Touch ID or Face ID authentication for sudo commands")]
    BiometricSudo(BiometricSudoArgs),

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

    /// Thoroughly remove files and reset configurations to defaults
    #[command(about = "Completely remove files and reset system configurations")]
    Purge(PurgeArgs),

    /// Restore system configurations and data from a previous backup
    #[command(about = "Restore system state from a previously created backup")]
    Restore(RestoreArgs),

    /// Manage the registry of configuration files and directories
    #[command(about = "Manage the registry of configuration files and directories")]
    RegistryConfigs(ConfigsRegistryArgs),

    /// Manage the registry of package managers for backup
    #[command(about = "Manage the registry of package managers for backup operations")]
    RegistryPackages(PackageRegistryArgs),

    /// Manage the registry of encrypted configuration files
    #[command(about = "Manage the registry of encrypted configuration files")]
    RegistryEncrypted(EncryptedRegistryArgs),

    /// Switch to a different profile
    #[command(about = "Switch to a different profile")]
    Use(UseArgs),

    /// Manage profiles (list, create, delete)
    #[command(about = "Manage profiles (list, create, delete)")]
    Profile(ProfileArgs),

    /// Synchronize configurations with a git repository
    #[command(about = "Sync configurations with a git repository (pull/push/both)")]
    Sync(SyncArgs),

    /// Validate configuration files and symlinks
    #[command(about = "Validate JSON configs, symlinks, and registry files")]
    Validate(ValidateArgs),

    /// Migrate legacy backup files to the new structure
    #[command(about = "Migrate legacy backup files to common/profiles structure")]
    Migrate(MigrateArgs),

    /// Interactive setup wizard for new users
    #[command(about = "Interactive wizard to configure mntn for your system")]
    Setup,
}
