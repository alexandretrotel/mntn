use crate::commands::core::Command;
use crate::profiles::ActiveProfile;
use crate::utils::paths::get_mntn_dir;
use std::fs;

mod config;
mod encrypted;
mod package;
mod utils;

struct BackupTask {
    profile: ActiveProfile,
    skip_encrypted: bool,
}

impl BackupTask {
    fn new(profile: ActiveProfile, skip_encrypted: bool) -> Self {
        Self {
            profile,
            skip_encrypted,
        }
    }
}

impl Command for BackupTask {
    fn name(&self) -> &str {
        "Backup"
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let mntn_dir = get_mntn_dir();
        crate::commands::git::init_repo_if_missing(&mntn_dir)?;

        let backup_path = self.profile.get_backup_path();
        fs::create_dir_all(&backup_path)?;

        println!("Backing up...");
        println!("   Target: {}", self.profile);

        let packages_path = crate::utils::paths::get_packages_path();
        fs::create_dir_all(&packages_path)?;

        config::backup_configs(&backup_path);
        package::backup_packages(&packages_path);

        if !self.skip_encrypted {
            let encrypted_backup_path = self.profile.get_encrypted_backup_path();
            fs::create_dir_all(&encrypted_backup_path)?;
            encrypted::backup_encrypted_configs(&encrypted_backup_path);
        }

        println!("Backup complete");
        Ok(())
    }
}

pub(crate) fn run(args: crate::cli::BackupArgs) {
    use crate::commands::core::CommandExecutor;

    let profile = args.resolve_profile();
    CommandExecutor::run(&mut BackupTask::new(profile, args.skip_encrypted));
}
