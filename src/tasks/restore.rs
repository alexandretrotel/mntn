use crate::logger::log;
use crate::profile::ActiveProfile;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::tasks::core::{PlannedOperation, Task};
use crate::utils::paths::get_registry_path;
use std::fs;
use std::path::{Path, PathBuf};

pub struct RestoreTask {
    profile: ActiveProfile,
}

impl RestoreTask {
    pub fn new(profile: ActiveProfile) -> Self {
        Self { profile }
    }
}

impl Task for RestoreTask {
    fn name(&self) -> &str {
        "Restore"
    }

    fn execute(&mut self) {
        println!("🔄 Starting restore process...");
        println!(
            "   Profile: machine={}, env={}",
            self.profile.machine_id, self.profile.environment
        );

        let registry_path = get_registry_path();
        let registry = match ConfigsRegistry::load_or_create(&registry_path) {
            Ok(registry) => registry,
            Err(e) => {
                println!("❌ Failed to load registry: {}", e);
                log(&format!("Failed to load registry: {}", e));
                return;
            }
        };

        let mut restored_count = 0;
        let mut skipped_count = 0;

        for (id, entry) in registry.get_enabled_entries() {
            let target_path = &entry.target_path;

            match self.profile.resolve_source(&entry.source_path) {
                Some(resolved) => {
                    println!("🔄 Restoring: {} ({}) [{}]", entry.name, id, resolved.layer);
                    if restore_config_file(&resolved.path, target_path, &entry.name) {
                        restored_count += 1;
                    } else {
                        skipped_count += 1;
                    }
                }
                None => {
                    println!("ℹ️ No backup found for {} in any layer", entry.name);
                    skipped_count += 1;
                }
            }
        }

        println!(
            "✅ Restore complete. {} restored, {} skipped.",
            restored_count, skipped_count
        );
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();

        if let Ok(registry) = ConfigsRegistry::load_or_create(&get_registry_path()) {
            for (_id, entry) in registry.get_enabled_entries() {
                let target_path = &entry.target_path;

                match self.profile.resolve_source(&entry.source_path) {
                    Some(resolved) => {
                        operations.push(PlannedOperation::with_target(
                            format!("Restore {} [{}]", entry.name, resolved.layer),
                            format!("{} -> {}", resolved.path.display(), target_path.display()),
                        ));
                    }
                    None => {
                        operations.push(PlannedOperation::with_target(
                            format!("Skip {} (no source)", entry.name),
                            format!("??? -> {}", target_path.display()),
                        ));
                    }
                }
            }
        }

        operations
    }
}

pub fn run_with_args(args: crate::cli::RestoreArgs) {
    use crate::tasks::core::TaskExecutor;
    let profile = ActiveProfile::from_defaults();
    TaskExecutor::run(&mut RestoreTask::new(profile), args.dry_run);
}

/// Attempts to restore a configuration file from a backup to its target location.
///
/// If the backup file exists and the target path is specified, this function:
/// - Reads the contents of the backup file.
/// - Creates parent directories for the target path if they don't exist.
/// - Writes the contents to the target path.
///
/// Returns true if the restore was successful, false otherwise.
fn restore_config_file(backup_path: &PathBuf, target_path: &PathBuf, file_name: &str) -> bool {
    // Handle both files and directories
    if backup_path.is_dir() {
        return restore_directory(backup_path, target_path, file_name);
    }

    let contents = match fs::read_to_string(backup_path) {
        Ok(c) => c,
        Err(e) => {
            println!("⚠️ Failed to read backup file for {}: {}", file_name, e);
            log(&format!(
                "Failed to read backup file for {}: {}",
                file_name, e
            ));
            return false;
        }
    };

    if let Some(parent) = target_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        println!("⚠️ Failed to create directory for {}: {}", file_name, e);
        log(&format!(
            "Failed to create directory for {}: {}",
            file_name, e
        ));
        return false;
    }

    match fs::write(target_path, contents) {
        Ok(_) => {
            log(&format!("Restored {}", file_name));
            true
        }
        Err(e) => {
            println!("⚠️ Failed to restore {}: {}", file_name, e);
            log(&format!("Failed to restore {}: {}", file_name, e));
            false
        }
    }
}

/// Restores a directory from backup to target location
fn restore_directory(backup_path: &Path, target_path: &Path, dir_name: &str) -> bool {
    use std::process::Command;

    // Create target directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(target_path) {
        println!(
            "⚠️ Failed to create target directory for {}: {}",
            dir_name, e
        );
        log(&format!(
            "Failed to create target directory for {}: {}",
            dir_name, e
        ));
        return false;
    }

    // Use rsync to copy directory contents
    let output = Command::new("rsync")
        .args(["-av", "--delete"])
        .arg(format!("{}/", backup_path.display())) // trailing slash for rsync
        .arg(target_path)
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                log(&format!("Restored directory {}", dir_name));
                true
            } else {
                let stderr = match String::from_utf8(result.stderr.clone()) {
                    Ok(s) => s,
                    Err(e) => format!("{:?}", e.into_bytes()),
                };

                println!("⚠️ Failed to restore directory {}: {}", dir_name, stderr);
                log(&format!(
                    "Failed to restore directory {}: {}",
                    dir_name, stderr
                ));
                false
            }
        }
        Err(e) => {
            println!("⚠️ Failed to run rsync for {}: {}", dir_name, e);
            log(&format!("Failed to run rsync for {}: {}", dir_name, e));
            false
        }
    }
}
