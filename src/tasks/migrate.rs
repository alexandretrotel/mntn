use crate::logger::{log_success, log_warning};
use crate::profile::ActiveProfile;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::tasks::core::{PlannedOperation, Task};
use crate::utils::filesystem::copy_dir_recursive;
use crate::utils::paths::{
    get_backup_common_path, get_backup_environment_path, get_backup_machine_path, get_backup_root,
    get_registry_path,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrateTarget {
    Common,
    Machine,
    Environment,
}

impl std::fmt::Display for MigrateTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateTarget::Common => write!(f, "common"),
            MigrateTarget::Machine => write!(f, "machine"),
            MigrateTarget::Environment => write!(f, "environment"),
        }
    }
}

impl MigrateTarget {
    pub fn resolve_path(&self, profile: &ActiveProfile) -> PathBuf {
        match self {
            MigrateTarget::Common => get_backup_common_path(),
            MigrateTarget::Machine => get_backup_machine_path(&profile.machine_id),
            MigrateTarget::Environment => get_backup_environment_path(&profile.environment),
        }
    }
}

pub struct MigrateTask {
    profile: ActiveProfile,
    target: MigrateTarget,
}

impl MigrateTask {
    pub fn new(profile: ActiveProfile, target: MigrateTarget) -> Self {
        Self { profile, target }
    }

    fn find_legacy_files(&self) -> Vec<(String, PathBuf)> {
        let registry_path = get_registry_path();
        let registry = match ConfigsRegistry::load_or_create(&registry_path) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        let mut legacy_files = Vec::new();
        let backup_root = get_backup_root();

        for (_id, entry) in registry.get_enabled_entries() {
            let legacy_path = backup_root.join(&entry.source_path);

            if legacy_path.exists() {
                let common_path = get_backup_common_path().join(&entry.source_path);
                let machine_path =
                    get_backup_machine_path(&self.profile.machine_id).join(&entry.source_path);
                let env_path =
                    get_backup_environment_path(&self.profile.environment).join(&entry.source_path);

                let is_in_layered = common_path.exists()
                    || machine_path.exists()
                    || env_path.exists()
                    || self.is_in_layered_subdir(&legacy_path);

                if !is_in_layered {
                    legacy_files.push((entry.source_path.clone(), legacy_path));
                }
            }
        }

        legacy_files
    }

    fn is_in_layered_subdir(&self, path: &Path) -> bool {
        let backup_root = get_backup_root();

        if let Ok(relative) = path.strip_prefix(&backup_root) {
            let first_component = relative
                .components()
                .next()
                .map(|c| c.as_os_str().to_string_lossy().to_string());

            matches!(
                first_component.as_deref(),
                Some("common") | Some("machines") | Some("environments")
            )
        } else {
            false
        }
    }
}

impl Task for MigrateTask {
    fn name(&self) -> &str {
        "Migrate"
    }

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔄 Migrating legacy backup files...");
        println!("   Target: {} ({})", self.target, self.profile);

        let target_dir = self.target.resolve_path(&self.profile);
        fs::create_dir_all(&target_dir)?;

        let legacy_files = self.find_legacy_files();

        if legacy_files.is_empty() {
            log_success("No legacy files found to migrate.");
            return Ok(());
        }

        println!("📋 Found {} legacy files to migrate", legacy_files.len());

        let mut migrated = 0;
        let mut failed = 0;

        for (source_path, legacy_path) in legacy_files {
            let new_path = target_dir.join(&source_path);

            if let Some(parent) = new_path.parent()
                && let Err(e) = fs::create_dir_all(parent)
            {
                log_warning(&format!(
                    "Failed to create parent dir for {}: {}",
                    source_path, e
                ));
                failed += 1;
                continue;
            }

            match move_path(&legacy_path, &new_path) {
                Ok(result) => {
                    if let Some(warning) = &result.removal_warning {
                        // Source removal failed - data is at destination but source still exists
                        log_warning(&format!(
                            "Migrated with warning: {} -> {}/{} ({})",
                            source_path, self.target, source_path, warning
                        ));
                    } else {
                        log_success(&format!(
                            "Migrated: {} -> {}/{}",
                            source_path, self.target, source_path
                        ));
                    }
                    migrated += 1;
                }
                Err(e) => {
                    log_warning(&format!("Failed to migrate {}: {}", source_path, e));
                    failed += 1;
                }
            }
        }

        log_success(&format!(
            "Migration complete. Migrated: {}, Failed: {}",
            migrated, failed
        ));

        Ok(())
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let target_dir = self.target.resolve_path(&self.profile);
        let legacy_files = self.find_legacy_files();

        for (source_path, legacy_path) in legacy_files {
            let new_path = target_dir.join(&source_path);
            operations.push(PlannedOperation::with_target(
                format!("Migrate to {}", self.target),
                format!("{} -> {}", legacy_path.display(), new_path.display()),
            ));
        }

        if operations.is_empty() {
            operations.push(PlannedOperation::with_target(
                "No migration needed".to_string(),
                "All files already in layered structure".to_string(),
            ));
        }

        operations
    }
}

/// Result of a move operation that may have partially succeeded
#[derive(Debug)]
pub struct MoveResult {
    /// Warning message if source removal failed (potential duplicate data)
    pub removal_warning: Option<String>,
}

impl MoveResult {
    fn ok() -> Self {
        Self {
            removal_warning: None,
        }
    }

    fn with_removal_warning(warning: String) -> Self {
        Self {
            removal_warning: Some(warning),
        }
    }
}

/// Counts the number of entries (files and directories) in a directory recursively.
fn count_entries(path: &Path) -> std::io::Result<usize> {
    let mut count = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            count += 1;
            if entry.path().is_dir() {
                count += count_entries(&entry.path())?;
            }
        }
    }
    Ok(count)
}

/// Moves a file or directory from `from` to `to`.
///
/// This function prefers `fs::rename` for atomic moves on the same filesystem.
/// If rename fails (e.g., cross-filesystem move), it falls back to copy + verify + remove.
///
/// The verification step ensures the destination contains the expected number of entries
/// before attempting to remove the source.
///
/// Returns a `MoveResult` that indicates success and any warnings about failed source removal.
fn move_path(from: &PathBuf, to: &PathBuf) -> std::io::Result<MoveResult> {
    // First, try atomic rename (works on same filesystem)
    match fs::rename(from, to) {
        Ok(()) => return Ok(MoveResult::ok()),
        Err(e) => {
            // EXDEV (cross-device link) or other errors mean we need to copy
            // Log at debug level that we're falling back to copy
            if e.raw_os_error() != Some(libc::EXDEV) {
                // For non-cross-device errors, still try copy as fallback
                // but the rename might have failed for permissions or other reasons
            }
        }
    }

    // Fallback: copy then remove
    if from.is_dir() {
        // Count source entries for verification
        let source_count = count_entries(from)?;

        // Create destination and copy
        fs::create_dir_all(to)?;
        copy_dir_recursive(from, to)?;

        // Verify destination has the expected entries
        let dest_count = count_entries(to)?;
        if dest_count != source_count {
            return Err(std::io::Error::other(format!(
                "Verification failed: source had {} entries but destination has {}",
                source_count, dest_count
            )));
        }

        // Attempt to remove source directory
        if let Err(e) = fs::remove_dir_all(from) {
            let warning = format!(
                "Failed to remove source directory '{}' after successful copy: {}. \
                 This may result in duplicate data.",
                from.display(),
                e
            );
            log_warning(&warning);
            return Ok(MoveResult::with_removal_warning(warning));
        }
    } else {
        // For files, copy then remove
        fs::copy(from, to)?;

        // Verify destination file exists and has same size
        let src_metadata = fs::metadata(from)?;
        let dst_metadata = fs::metadata(to)?;
        if src_metadata.len() != dst_metadata.len() {
            return Err(std::io::Error::other(format!(
                "Verification failed: source file size ({}) differs from destination ({})",
                src_metadata.len(),
                dst_metadata.len()
            )));
        }

        // Attempt to remove source file
        if let Err(e) = fs::remove_file(from) {
            let warning = format!(
                "Failed to remove source file '{}' after successful copy: {}. \
                 This may result in duplicate data.",
                from.display(),
                e
            );
            log_warning(&warning);
            return Ok(MoveResult::with_removal_warning(warning));
        }
    }

    Ok(MoveResult::ok())
}

pub fn run_with_args(args: crate::cli::MigrateArgs) {
    use crate::tasks::core::TaskExecutor;

    let profile = args.profile_args.resolve();
    let target = args.layer.to_migrate_target();

    TaskExecutor::run(&mut MigrateTask::new(profile, target), args.dry_run);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ActiveProfile;
    use tempfile::TempDir;

    fn create_test_profile() -> ActiveProfile {
        ActiveProfile {
            name: None,
            machine_id: "test-machine".to_string(),
            environment: "test-env".to_string(),
        }
    }

    #[test]
    fn test_migrate_target_display() {
        assert_eq!(MigrateTarget::Common.to_string(), "common");
        assert_eq!(MigrateTarget::Machine.to_string(), "machine");
        assert_eq!(MigrateTarget::Environment.to_string(), "environment");
    }

    #[test]
    fn test_migrate_target_equality() {
        assert_eq!(MigrateTarget::Common, MigrateTarget::Common);
        assert_ne!(MigrateTarget::Common, MigrateTarget::Machine);
        assert_ne!(MigrateTarget::Machine, MigrateTarget::Environment);
    }

    #[test]
    fn test_migrate_target_clone() {
        let target = MigrateTarget::Machine;
        let cloned = target;
        assert_eq!(target, cloned);
    }

    #[test]
    fn test_migrate_target_resolve_path_common() {
        let profile = create_test_profile();
        let path = MigrateTarget::Common.resolve_path(&profile);
        assert!(path.to_string_lossy().contains("common"));
    }

    #[test]
    fn test_migrate_target_resolve_path_machine() {
        let profile = create_test_profile();
        let path = MigrateTarget::Machine.resolve_path(&profile);
        assert!(path.to_string_lossy().contains("machines"));
        assert!(path.to_string_lossy().contains("test-machine"));
    }

    #[test]
    fn test_migrate_target_resolve_path_environment() {
        let profile = create_test_profile();
        let path = MigrateTarget::Environment.resolve_path(&profile);
        assert!(path.to_string_lossy().contains("environments"));
        assert!(path.to_string_lossy().contains("test-env"));
    }

    #[test]
    fn test_migrate_target_resolve_path_different_profiles() {
        let profile1 = ActiveProfile {
            name: None,
            machine_id: "machine-1".to_string(),
            environment: "env-1".to_string(),
        };
        let profile2 = ActiveProfile {
            name: None,
            machine_id: "machine-2".to_string(),
            environment: "env-2".to_string(),
        };

        let path1 = MigrateTarget::Machine.resolve_path(&profile1);
        let path2 = MigrateTarget::Machine.resolve_path(&profile2);
        assert_ne!(path1, path2);

        let path1 = MigrateTarget::Environment.resolve_path(&profile1);
        let path2 = MigrateTarget::Environment.resolve_path(&profile2);
        assert_ne!(path1, path2);
    }

    #[test]
    fn test_migrate_task_name() {
        let task = MigrateTask::new(create_test_profile(), MigrateTarget::Common);
        assert_eq!(task.name(), "Migrate");
    }

    #[test]
    fn test_migrate_task_new() {
        let profile = create_test_profile();
        let task = MigrateTask::new(profile.clone(), MigrateTarget::Machine);
        assert_eq!(task.profile.machine_id, profile.machine_id);
        assert_eq!(task.target, MigrateTarget::Machine);
    }

    #[test]
    fn test_migrate_task_dry_run() {
        let task = MigrateTask::new(create_test_profile(), MigrateTarget::Common);
        // Should not panic
        let ops = task.dry_run();
        // Should return at least one operation (even if just "no migration needed")
        assert!(!ops.is_empty());
    }

    #[test]
    fn test_migrate_task_dry_run_has_target() {
        let task = MigrateTask::new(create_test_profile(), MigrateTarget::Environment);
        let ops = task.dry_run();

        // All operations should have targets
        for op in &ops {
            assert!(op.target.is_some());
        }
    }

    #[test]
    fn test_move_path_file() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("source.txt");
        let to = temp_dir.path().join("dest.txt");

        fs::write(&from, "content").unwrap();

        let result = move_path(&from, &to);
        assert!(result.is_ok());
        let move_result = result.unwrap();
        assert!(move_result.removal_warning.is_none());

        // Source should be gone
        assert!(!from.exists());
        // Destination should exist with content
        assert!(to.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "content");
    }

    #[test]
    fn test_move_path_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("source_dir");
        let to = temp_dir.path().join("dest_dir");

        fs::create_dir(&from).unwrap();
        fs::write(from.join("file.txt"), "dir content").unwrap();

        let result = move_path(&from, &to);
        assert!(result.is_ok());
        let move_result = result.unwrap();
        assert!(move_result.removal_warning.is_none());

        // Source should be gone
        assert!(!from.exists());
        // Destination should exist with content
        assert!(to.exists());
        assert!(to.is_dir());
        assert_eq!(
            fs::read_to_string(to.join("file.txt")).unwrap(),
            "dir content"
        );
    }

    #[test]
    fn test_move_path_nested_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("source_dir");
        let to = temp_dir.path().join("dest_dir");

        // Create nested structure
        fs::create_dir_all(from.join("sub").join("deep")).unwrap();
        fs::write(
            from.join("sub").join("deep").join("file.txt"),
            "deep content",
        )
        .unwrap();

        let result = move_path(&from, &to);
        assert!(result.is_ok());

        assert!(!from.exists());
        assert!(to.join("sub").join("deep").join("file.txt").exists());
    }

    #[test]
    fn test_move_path_source_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("nonexistent.txt");
        let to = temp_dir.path().join("dest.txt");

        let result = move_path(&from, &to);
        assert!(result.is_err());
    }

    #[test]
    fn test_move_path_uses_rename_on_same_filesystem() {
        // When source and destination are on the same filesystem,
        // rename should be used (atomic operation)
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("source.txt");
        let to = temp_dir.path().join("dest.txt");

        fs::write(&from, "atomic test").unwrap();

        let result = move_path(&from, &to);
        assert!(result.is_ok());

        // Verify the move happened
        assert!(!from.exists());
        assert!(to.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "atomic test");
    }

    #[test]
    fn test_count_entries_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let count = count_entries(temp_dir.path()).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_entries_with_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "a").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "b").unwrap();

        let count = count_entries(temp_dir.path()).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_entries_nested() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("nested.txt"), "nested").unwrap();
        fs::write(temp_dir.path().join("root.txt"), "root").unwrap();

        let count = count_entries(temp_dir.path()).unwrap();
        // 1 (subdir) + 1 (nested.txt) + 1 (root.txt) = 3
        assert_eq!(count, 3);
    }

    #[test]
    fn test_move_result_ok() {
        let result = MoveResult::ok();
        assert!(result.removal_warning.is_none());
    }

    #[test]
    fn test_move_result_with_warning() {
        let result = MoveResult::with_removal_warning("test warning".to_string());
        assert!(result.removal_warning.is_some());
        assert_eq!(result.removal_warning.unwrap(), "test warning");
    }

    #[test]
    fn test_is_in_layered_subdir_common() {
        let profile = create_test_profile();
        let task = MigrateTask::new(profile, MigrateTarget::Common);

        let backup_root = get_backup_root();
        let common_path = backup_root.join("common").join("some_file.txt");

        assert!(task.is_in_layered_subdir(&common_path));
    }

    #[test]
    fn test_is_in_layered_subdir_machines() {
        let profile = create_test_profile();
        let task = MigrateTask::new(profile, MigrateTarget::Common);

        let backup_root = get_backup_root();
        let machine_path = backup_root
            .join("machines")
            .join("my-machine")
            .join("file.txt");

        assert!(task.is_in_layered_subdir(&machine_path));
    }

    #[test]
    fn test_is_in_layered_subdir_environments() {
        let profile = create_test_profile();
        let task = MigrateTask::new(profile, MigrateTarget::Common);

        let backup_root = get_backup_root();
        let env_path = backup_root
            .join("environments")
            .join("prod")
            .join("file.txt");

        assert!(task.is_in_layered_subdir(&env_path));
    }

    #[test]
    fn test_is_in_layered_subdir_legacy() {
        let profile = create_test_profile();
        let task = MigrateTask::new(profile, MigrateTarget::Common);

        let backup_root = get_backup_root();
        let legacy_path = backup_root.join("some_legacy_file.txt");

        assert!(!task.is_in_layered_subdir(&legacy_path));
    }

    #[test]
    fn test_is_in_layered_subdir_outside_backup() {
        let profile = create_test_profile();
        let task = MigrateTask::new(profile, MigrateTarget::Common);

        let outside_path = PathBuf::from("/tmp/some_file.txt");

        assert!(!task.is_in_layered_subdir(&outside_path));
    }

    #[test]
    fn test_find_legacy_files_empty() {
        let profile = create_test_profile();
        let task = MigrateTask::new(profile, MigrateTarget::Common);

        // Should not panic - just verify it returns successfully
        let _legacy = task.find_legacy_files();
    }

    #[test]
    fn test_migrate_task_all_targets() {
        let profile = create_test_profile();

        for target in [
            MigrateTarget::Common,
            MigrateTarget::Machine,
            MigrateTarget::Environment,
        ] {
            let task = MigrateTask::new(profile.clone(), target);
            assert_eq!(task.name(), "Migrate");
            let ops = task.dry_run();
            assert!(!ops.is_empty());
        }
    }
}
