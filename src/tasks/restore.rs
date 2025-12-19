use crate::logger::{log, log_info, log_success, log_warning};
use crate::profile::ActiveProfile;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::tasks::core::{PlannedOperation, Task};
use crate::utils::paths::get_registry_path;
use crate::utils::system::rsync_directory;
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

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔄 Starting restore process...");
        println!("   Profile: {}", self.profile);

        let registry_path = get_registry_path();
        let registry = ConfigsRegistry::load_or_create(&registry_path)?;

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
                    log_info(&format!("No backup found for {} in any layer", entry.name));
                    skipped_count += 1;
                }
            }
        }

        log_success(&format!(
            "Restore complete. {} restored, {} skipped",
            restored_count, skipped_count
        ));

        Ok(())
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
    let profile = args.resolve_profile();
    TaskExecutor::run(&mut RestoreTask::new(profile), args.dry_run);
}

/// Attempts to restore a configuration file from a backup to its target location.
///
/// If the backup file exists and the target path is specified, this function:
/// - Removes any existing symlink at target (legacy migration support).
/// - Reads the contents of the backup file.
/// - Creates parent directories for the target path if they don't exist.
/// - Writes the contents to the target path.
///
/// Returns true if the restore was successful, false otherwise.
fn restore_config_file(backup_path: &PathBuf, target_path: &PathBuf, file_name: &str) -> bool {
    if backup_path.is_dir() {
        return restore_directory(backup_path, target_path, file_name);
    }

    // Handle legacy symlinks: if target is a symlink (from old system), remove it first
    if target_path.is_symlink() {
        if let Err(e) = fs::remove_file(target_path) {
            log_warning(&format!(
                "Failed to remove legacy symlink for {}: {}",
                file_name, e
            ));
            return false;
        }
        log(&format!(
            "Removed legacy symlink at {}",
            target_path.display()
        ));
    }

    // Use fs::read instead of fs::read_to_string to handle binary files
    let contents = match fs::read(backup_path) {
        Ok(c) => c,
        Err(e) => {
            log_warning(&format!(
                "Failed to read backup file for {}: {}",
                file_name, e
            ));
            return false;
        }
    };

    if let Some(parent) = target_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        log_warning(&format!(
            "Failed to create directory for {}: {}",
            file_name, e
        ));
        return false;
    }

    match fs::write(target_path, contents) {
        Ok(()) => {
            log(&format!("Restored {}", file_name));
            true
        }
        Err(e) => {
            log_warning(&format!("Failed to restore {}: {}", file_name, e));
            false
        }
    }
}

/// Restores a directory from backup to target location.
/// If target is a symlink (legacy), removes it first and creates a real directory.
fn restore_directory(backup_path: &Path, target_path: &Path, dir_name: &str) -> bool {
    // Handle legacy symlinks: if target is a symlink (from old system), remove it first
    if target_path.is_symlink() {
        if let Err(e) = fs::remove_file(target_path) {
            log_warning(&format!(
                "Failed to remove legacy symlink for {}: {}",
                dir_name, e
            ));
            return false;
        }
        log(&format!(
            "Removed legacy symlink at {}",
            target_path.display()
        ));
    }

    if let Err(e) = fs::create_dir_all(target_path) {
        log_warning(&format!(
            "Failed to create target directory for {}: {}",
            dir_name, e
        ));
        return false;
    }

    match rsync_directory(backup_path, target_path) {
        Ok(()) => {
            log(&format!("Restored directory {}", dir_name));
            true
        }
        Err(e) => {
            log_warning(&format!("Failed to restore directory {}: {}", dir_name, e));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ActiveProfile;
    use tempfile::TempDir;

    fn create_test_profile() -> ActiveProfile {
        ActiveProfile::with_profile("test-profile")
    }

    #[test]
    fn test_restore_task_name() {
        let task = RestoreTask::new(create_test_profile());
        assert_eq!(task.name(), "Restore");
    }

    #[test]
    fn test_restore_task_new() {
        let profile = create_test_profile();
        let task = RestoreTask::new(profile.clone());
        assert_eq!(task.profile.name, profile.name);
    }

    #[test]
    fn test_restore_task_dry_run() {
        let task = RestoreTask::new(create_test_profile());
        // Should not panic - just verify it returns successfully
        let _ops = task.dry_run();
    }

    #[test]
    fn test_restore_config_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let backup_path = temp_dir.path().join("backup.txt");
        let target_path = temp_dir.path().join("target.txt");

        fs::write(&backup_path, "backup content").unwrap();

        let result = restore_config_file(&backup_path, &target_path, "test-file");
        assert!(result);

        assert!(target_path.exists());
        assert_eq!(fs::read_to_string(&target_path).unwrap(), "backup content");
    }

    #[test]
    fn test_restore_config_file_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let backup_path = temp_dir.path().join("backup.txt");
        let target_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("target.txt");

        fs::write(&backup_path, "content").unwrap();

        let result = restore_config_file(&backup_path, &target_path, "test-file");
        assert!(result);

        assert!(target_path.exists());
    }

    #[test]
    fn test_restore_config_file_backup_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let backup_path = temp_dir.path().join("nonexistent.txt");
        let target_path = temp_dir.path().join("target.txt");

        let result = restore_config_file(&backup_path, &target_path, "test-file");
        assert!(!result);

        assert!(!target_path.exists());
    }

    #[test]
    fn test_restore_config_file_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let backup_path = temp_dir.path().join("backup.txt");
        let target_path = temp_dir.path().join("target.txt");

        fs::write(&backup_path, "new content").unwrap();
        fs::write(&target_path, "old content").unwrap();

        let result = restore_config_file(&backup_path, &target_path, "test-file");
        assert!(result);

        assert_eq!(fs::read_to_string(&target_path).unwrap(), "new content");
    }

    #[test]
    fn test_restore_config_file_directory() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backup_dir");
        let target_dir = temp_dir.path().join("target_dir");

        // Create backup directory with content
        fs::create_dir(&backup_dir).unwrap();
        fs::write(backup_dir.join("file.txt"), "directory content").unwrap();

        let result = restore_config_file(&backup_dir, &target_dir, "test-dir");

        // May fail without rsync, but should handle gracefully
        // Just check it doesn't panic - result is always a bool
        let _ = result;
    }

    #[test]
    fn test_restore_directory_creates_target() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backup_dir");
        let target_dir = temp_dir.path().join("target_dir");

        fs::create_dir(&backup_dir).unwrap();

        // Will fail without rsync but should create target dir
        let _ = restore_directory(&backup_dir, &target_dir, "test-dir");

        // Target directory should be created even if rsync fails
        assert!(target_dir.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_restore_directory_success() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backup_dir");
        let target_dir = temp_dir.path().join("target_dir");

        // Create backup directory with content
        fs::create_dir(&backup_dir).unwrap();
        fs::write(backup_dir.join("file.txt"), "content").unwrap();

        let result = restore_directory(&backup_dir, &target_dir, "test-dir");

        // Skip if rsync not available
        if !result && !target_dir.join("file.txt").exists() {
            return; // rsync not available
        }

        assert!(result);
        assert!(target_dir.join("file.txt").exists());
    }

    #[test]
    fn test_restore_directory_nested_target() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backup_dir");
        let target_dir = temp_dir.path().join("nested").join("target_dir");

        fs::create_dir(&backup_dir).unwrap();

        let _ = restore_directory(&backup_dir, &target_dir, "test-dir");

        // Target and its parents should be created
        assert!(target_dir.exists());
    }

    #[test]
    fn test_restore_task_profile_display() {
        let profile = ActiveProfile::with_profile("test-profile");
        let task = RestoreTask::new(profile);

        // Profile should be stored correctly
        assert_eq!(task.profile.name, Some("test-profile".to_string()));
    }

    #[test]
    fn test_restore_config_file_binary() {
        let temp_dir = TempDir::new().unwrap();
        let backup_path = temp_dir.path().join("backup.bin");
        let target_path = temp_dir.path().join("target.bin");

        // Binary content with non-UTF-8 bytes
        let binary_content: Vec<u8> = vec![0x00, 0x01, 0xFF, 0xFE, 0x89, 0x50, 0x4E, 0x47];
        fs::write(&backup_path, &binary_content).unwrap();

        let result = restore_config_file(&backup_path, &target_path, "test-binary");
        assert!(result);

        assert!(target_path.exists());
        assert_eq!(fs::read(&target_path).unwrap(), binary_content);
    }

    #[test]
    #[cfg(unix)]
    fn test_restore_config_file_replaces_symlink() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let backup_path = temp_dir.path().join("backup.txt");
        let target_path = temp_dir.path().join("target.txt");
        let symlink_target = temp_dir.path().join("old_target.txt");

        // Create backup content
        fs::write(&backup_path, "new content from backup").unwrap();

        // Create a symlink at target location (simulating legacy setup)
        fs::write(&symlink_target, "old symlink content").unwrap();
        symlink(&symlink_target, &target_path).unwrap();
        assert!(target_path.is_symlink());

        let result = restore_config_file(&backup_path, &target_path, "test-file");
        assert!(result);

        // Target should now be a real file, not a symlink
        assert!(!target_path.is_symlink());
        assert!(target_path.is_file());
        assert_eq!(
            fs::read_to_string(&target_path).unwrap(),
            "new content from backup"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_restore_directory_replaces_symlink() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backup_dir");
        let target_path = temp_dir.path().join("target_dir");
        let symlink_target = temp_dir.path().join("old_target_dir");

        // Create backup directory with content
        fs::create_dir(&backup_dir).unwrap();
        fs::write(backup_dir.join("file.txt"), "backup content").unwrap();

        // Create a symlink at target location (simulating legacy setup)
        fs::create_dir(&symlink_target).unwrap();
        symlink(&symlink_target, &target_path).unwrap();
        assert!(target_path.is_symlink());

        let result = restore_directory(&backup_dir, &target_path, "test-dir");

        // Skip if rsync not available
        if !result {
            return;
        }

        // Target should now be a real directory, not a symlink
        assert!(!target_path.is_symlink());
        assert!(target_path.is_dir());
    }
}
