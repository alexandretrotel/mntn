use clap::{Args, Parser, Subcommand};

use crate::profiles::ActiveProfile;

#[derive(Parser)]
#[command(
    name = "mntn",
    version = env!("CARGO_PKG_VERSION"),
    about = "A Rust-based command-line tool for dotfiles management with profiles."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Args)]
pub struct BackupArgs {
    #[arg(long, short = 'p', help = "Target a specific profile for backup")]
    pub profile: Option<String>,
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

#[derive(Args)]
pub struct RestoreArgs {
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

#[derive(Args)]
pub struct ValidateArgs {}

impl ValidateArgs {
    pub fn resolve_profile(&self) -> ActiveProfile {
        ActiveProfile::resolve(None)
    }
}

#[derive(Args)]
pub struct GitArgs {
    #[arg(trailing_var_arg = true, required = true)]
    pub args: Vec<String>,
}

#[derive(Args)]
pub struct UseArgs {
    #[arg(help = "Profile name to switch to")]
    pub profile: String,
}

#[derive(Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub action: Option<ProfileActions>,
}

#[derive(Subcommand)]
pub enum ProfileActions {
    #[command(about = "List all available profiles")]
    List,

    #[command(about = "Create a new profile")]
    Create {
        #[arg(help = "Name for the new profile")]
        name: String,
        #[arg(long, short = 'd', help = "Optional description for the profile")]
        description: Option<String>,
    },

    #[command(about = "Delete a profile")]
    Delete {
        #[arg(help = "Name of the profile to delete")]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Backup system configurations and user data to a safe location")]
    Backup(BackupArgs),

    #[command(about = "Restore system state from a previously created backup")]
    Restore(RestoreArgs),

    #[command(about = "Switch to a different profile")]
    Use(UseArgs),

    #[command(about = "Manage profiles (list, create, delete)")]
    Profile(ProfileArgs),

    #[command(about = "Run git commands in the mntn repository")]
    Git(GitArgs),

    #[command(about = "Validate JSON configs, symlinks, and registry files")]
    Validate(ValidateArgs),
}
