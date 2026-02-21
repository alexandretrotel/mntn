use crate::commands::core::Command;
use crate::encryption::prompt_password;
use crate::profiles::ActiveProfile;
use crate::registry::config::ConfigRegistry;
use crate::utils::paths::get_config_registry_path;

mod config;
mod encrypted;

struct RestoreTask {
    profile: ActiveProfile,
    skip_encrypted: bool,
}

impl RestoreTask {
    fn new(profile: ActiveProfile, skip_encrypted: bool) -> Self {
        Self {
            profile,
            skip_encrypted,
        }
    }
}

impl Command for RestoreTask {
    fn name(&self) -> &str {
        "Restore"
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        println!("Starting restore process...");
        println!("   Profile: {}", self.profile);

        let config_registry_path = get_config_registry_path();
        let config_registry = ConfigRegistry::load_or_create(&config_registry_path)?;

        let mut restored_count = 0;
        let mut skipped_count = 0;

        for (id, entry) in config_registry.get_enabled_entries() {
            let target_path = &entry.target_path;

            match self.profile.resolve_source(&entry.source_path) {
                Some(resolved) => {
                    println!("Restoring: {} ({}) [{}]", entry.name, id, resolved.layer);
                    if config::restore_configs(&resolved.path, target_path, &entry.name) {
                        restored_count += 1;
                    } else {
                        skipped_count += 1;
                    }
                }
                None => {
                    println!("No backup found for {} in any layer", entry.name);
                    skipped_count += 1;
                }
            }
        }

        if !self.skip_encrypted {
            match prompt_password(false) {
                Ok(password) => {
                    let (encrypted_restored, encrypted_skipped) =
                        encrypted::restore_encrypted_configs(&self.profile, &password);
                    restored_count += encrypted_restored;
                    skipped_count += encrypted_skipped;
                }
                Err(e) => {
                    eprintln!("Skipping encrypted restore: {}", e);
                }
            }
        }

        println!(
            "Restore complete. {} restored, {} skipped",
            restored_count, skipped_count
        );

        Ok(())
    }
}

pub(crate) fn run(args: crate::cli::RestoreArgs) {
    use crate::commands::core::CommandExecutor;
    let profile = args.resolve_profile();
    CommandExecutor::run(&mut RestoreTask::new(profile, args.skip_encrypted));
}
