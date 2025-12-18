use crate::logger::log;
use crate::profile::ActiveProfile;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::tasks::core::{PlannedOperation, Task};
use crate::utils::filesystem::{backup_existing_target, copy_dir_to_source};
use crate::utils::paths::{get_registry_path, get_symlinks_path};
use std::fs;
use std::path::Path;

pub struct LinkTask {
    profile: ActiveProfile,
}

impl LinkTask {
    pub fn new(profile: ActiveProfile) -> Self {
        Self { profile }
    }
}

impl Task for LinkTask {
    fn name(&self) -> &str {
        "Link"
    }

    fn execute(&mut self) {
        println!("🔗 Creating symlinks...");
        println!("   Profile: {}", self.profile);

        let symlinks_dir = get_symlinks_path();
        if let Err(e) = fs::create_dir_all(&symlinks_dir) {
            println!("Failed to create symlinks directory: {e}");
            log(&format!("Failed to create symlinks directory: {e}"));
            return;
        }

        let registry_path = get_registry_path();
        let registry = match ConfigsRegistry::load_or_create(&registry_path) {
            Ok(registry) => registry,
            Err(e) => {
                println!("❌ Failed to load registry: {}", e);
                log(&format!("Failed to load registry: {}", e));
                return;
            }
        };

        let mut links_processed = 0;
        let mut links_skipped = 0;
        let links_total = registry.get_enabled_entries().count();

        if links_total == 0 {
            println!("ℹ️ No enabled entries found in registry.");
            return;
        }

        println!("📋 Found {} enabled entries in registry", links_total);

        for (id, entry) in registry.get_enabled_entries() {
            let dst = &entry.target_path;

            match self.profile.resolve_source(&entry.source_path) {
                Some(resolved) => {
                    println!(
                        "🔗 Processing: {} ({}) [{}]",
                        entry.name, id, resolved.layer
                    );
                    process_link(&resolved.path, dst, &symlinks_dir);
                    links_processed += 1;
                }
                None => {
                    println!(
                        "⚠️  Skipping: {} ({}) - no source found in any layer",
                        entry.name, id
                    );
                    log(&format!(
                        "No source found for {} in any layer (checked: environment/{}, machines/{}, common, legacy)",
                        entry.source_path, self.profile.environment, self.profile.machine_id
                    ));
                    links_skipped += 1;
                }
            }
        }

        println!(
            "✅ Symlink creation complete. Processed: {}, Skipped: {}",
            links_processed, links_skipped
        );
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();

        if let Ok(registry) = ConfigsRegistry::load_or_create(&get_registry_path()) {
            for (_id, entry) in registry.get_enabled_entries() {
                let dst = &entry.target_path;

                match self.profile.resolve_source(&entry.source_path) {
                    Some(resolved) => {
                        operations.push(PlannedOperation::with_target(
                            format!("Link {} [{}]", entry.name, resolved.layer),
                            format!("{} -> {}", dst.display(), resolved.path.display()),
                        ));
                    }
                    None => {
                        operations.push(PlannedOperation::with_target(
                            format!("Skip {} (no source)", entry.name),
                            format!("{} -> ???", dst.display()),
                        ));
                    }
                }
            }
        }

        operations
    }
}

pub fn run_with_args(args: crate::cli::LinkArgs) {
    use crate::tasks::core::TaskExecutor;

    let profile = args.profile_args.resolve();

    TaskExecutor::run(&mut LinkTask::new(profile), args.dry_run);
}

/// Copies from dst to src if src is missing, handling both files and directories
fn copy_dst_to_src_if_missing(src: &Path, dst: &Path) -> Result<(), ()> {
    if dst.exists() && !dst.is_symlink() && !src.exists() {
        if dst.is_file() {
            fs::copy(dst, src).map_err(|e| {
                log(&format!(
                    "Failed to copy file {} to source {}: {}",
                    dst.display(),
                    src.display(),
                    e
                ));
            })?;
        } else if dst.is_dir() {
            copy_dir_to_source(dst, src).map_err(|e| {
                log(&format!(
                    "Failed to copy directory {} to source {}: {}",
                    dst.display(),
                    src.display(),
                    e
                ));
            })?;
        } else {
            log(&format!(
                "Unknown target type for {}. Skipping.",
                dst.display()
            ));
            return Err(());
        }
    }
    Ok(())
}

/// Handles existing symlink logic: checks if it's correct, removes if wrong
fn handle_existing_symlink(src: &Path, dst: &Path) -> Result<(), ()> {
    if dst.is_symlink() {
        match fs::read_link(dst) {
            Ok(existing) if existing == src => {
                log(&format!(
                    "Symlink {} already correctly points to {}",
                    dst.display(),
                    src.display()
                ));
                return Err(()); // nothing more to do
            }
            Ok(existing) => {
                log(&format!(
                    "Removing incorrect symlink {} → {}",
                    dst.display(),
                    existing.display()
                ));
                fs::remove_file(dst).map_err(|e| {
                    log(&format!(
                        "Failed to remove incorrect symlink {}: {}",
                        dst.display(),
                        e
                    ));
                })?;
            }
            Err(e) => {
                log(&format!("Failed to read symlink {}: {}", dst.display(), e));
                return Err(());
            }
        }
    }
    Ok(())
}

/// Backs up the destination if it exists and is not a symlink
fn backup_if_needed(dst: &Path, symlinks_dir: &Path) -> Result<(), ()> {
    if dst.exists() && !dst.is_symlink() {
        backup_existing_target(dst, symlinks_dir).map_err(|e| {
            log(&format!("Failed to back up {}: {}", dst.display(), e));
        })?;
    }
    Ok(())
}

/// Creates a symlink from src to dst
fn create_symlink(src: &Path, dst: &Path) {
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(src, dst);

    #[cfg(windows)]
    let result = if src.is_dir() {
        std::os::windows::fs::symlink_dir(src, dst)
    } else {
        std::os::windows::fs::symlink_file(src, dst)
    };

    match result {
        Ok(()) => log(&format!("Linked {} → {}", src.display(), dst.display())),
        Err(e) => log(&format!(
            "Failed to link {} → {}: {}",
            src.display(),
            dst.display(),
            e
        )),
    }
}

/// Processes a single (src, dst) link
fn process_link(src: &Path, dst: &Path, symlinks_dir: &Path) {
    if copy_dst_to_src_if_missing(src, dst).is_err() {
        return;
    }

    if !src.exists() {
        log(&format!(
            "Warning: Source {} does not exist. Skipping...",
            src.display()
        ));
        return;
    }

    if handle_existing_symlink(src, dst).is_err() {
        return;
    }

    if backup_if_needed(dst, symlinks_dir).is_err() {
        return;
    }

    create_symlink(src, dst);
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
    fn test_link_task_name() {
        let task = LinkTask::new(create_test_profile());
        assert_eq!(task.name(), "Link");
    }

    #[test]
    fn test_link_task_dry_run() {
        let task = LinkTask::new(create_test_profile());
        // Should not panic even without a valid registry
        let _ops = task.dry_run();
    }

    #[test]
    fn test_copy_dst_to_src_if_missing_src_exists() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst.txt");

        // Create both source and destination
        fs::write(&src, "source content").unwrap();
        fs::write(&dst, "destination content").unwrap();

        // Should do nothing if source exists
        let result = copy_dst_to_src_if_missing(&src, &dst);
        assert!(result.is_ok());

        // Source should remain unchanged
        assert_eq!(fs::read_to_string(&src).unwrap(), "source content");
    }

    #[test]
    fn test_copy_dst_to_src_if_missing_copies_file() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst.txt");

        // Only destination exists
        fs::write(&dst, "destination content").unwrap();

        // Should copy destination to source
        let result = copy_dst_to_src_if_missing(&src, &dst);
        assert!(result.is_ok());

        // Source should now exist with destination content
        assert!(src.exists());
        assert_eq!(fs::read_to_string(&src).unwrap(), "destination content");
    }

    #[test]
    fn test_copy_dst_to_src_if_missing_copies_directory() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src_dir");
        let dst = temp_dir.path().join("dst_dir");

        // Create destination directory with content
        fs::create_dir(&dst).unwrap();
        fs::write(dst.join("file.txt"), "directory content").unwrap();

        // Should copy destination directory to source
        let result = copy_dst_to_src_if_missing(&src, &dst);
        assert!(result.is_ok());

        // Source should now exist with destination content
        assert!(src.exists());
        assert!(src.is_dir());
        assert_eq!(
            fs::read_to_string(src.join("file.txt")).unwrap(),
            "directory content"
        );
    }

    #[test]
    fn test_copy_dst_to_src_if_missing_dst_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst.txt");

        // Neither exists
        let result = copy_dst_to_src_if_missing(&src, &dst);
        assert!(result.is_ok());

        // Nothing should be created
        assert!(!src.exists());
        assert!(!dst.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_copy_dst_to_src_if_missing_skips_symlink() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let real_file = temp_dir.path().join("real.txt");
        let dst = temp_dir.path().join("dst_link");

        // Create a symlink destination
        fs::write(&real_file, "real content").unwrap();
        symlink(&real_file, &dst).unwrap();

        // Should not copy symlink
        let result = copy_dst_to_src_if_missing(&src, &dst);
        assert!(result.is_ok());

        // Source should not exist
        assert!(!src.exists());
    }

    #[test]
    fn test_handle_existing_symlink_not_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst.txt");

        // Create regular file
        fs::write(&dst, "content").unwrap();

        // Should return Ok (not a symlink, nothing to handle)
        let result = handle_existing_symlink(&src, &dst);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn test_handle_existing_symlink_correct_target() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst_link");

        fs::write(&src, "source content").unwrap();
        symlink(&src, &dst).unwrap();

        // Should return Err (symlink is already correct, nothing more to do)
        let result = handle_existing_symlink(&src, &dst);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_handle_existing_symlink_wrong_target() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let wrong_target = temp_dir.path().join("wrong.txt");
        let dst = temp_dir.path().join("dst_link");

        fs::write(&wrong_target, "wrong content").unwrap();
        symlink(&wrong_target, &dst).unwrap();

        // Should return Ok and remove incorrect symlink
        let result = handle_existing_symlink(&src, &dst);
        assert!(result.is_ok());

        // Symlink should be removed
        assert!(!dst.exists());
    }

    #[test]
    fn test_backup_if_needed_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let dst = temp_dir.path().join("dst.txt");
        let symlinks_dir = temp_dir.path().join("symlinks");

        fs::write(&dst, "content to backup").unwrap();

        let result = backup_if_needed(&dst, &symlinks_dir);
        assert!(result.is_ok());

        // Original should be moved
        assert!(!dst.exists());

        // Backup should exist
        assert!(symlinks_dir.exists());
        let entries: Vec<_> = fs::read_dir(&symlinks_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_backup_if_needed_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let dst = temp_dir.path().join("nonexistent.txt");
        let symlinks_dir = temp_dir.path().join("symlinks");

        let result = backup_if_needed(&dst, &symlinks_dir);
        assert!(result.is_ok());

        // Symlinks dir should not be created
        assert!(!symlinks_dir.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_backup_if_needed_skips_symlink() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target.txt");
        let dst = temp_dir.path().join("dst_link");
        let symlinks_dir = temp_dir.path().join("symlinks");

        fs::write(&target, "target content").unwrap();
        symlink(&target, &dst).unwrap();

        let result = backup_if_needed(&dst, &symlinks_dir);
        assert!(result.is_ok());

        // Symlink should still exist
        assert!(dst.is_symlink());

        // Symlinks dir should not be created
        assert!(!symlinks_dir.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_create_symlink_file() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst_link");

        fs::write(&src, "source content").unwrap();

        create_symlink(&src, &dst);

        assert!(dst.is_symlink());
        assert_eq!(fs::read_link(&dst).unwrap(), src);
        assert_eq!(fs::read_to_string(&dst).unwrap(), "source content");
    }

    #[test]
    #[cfg(unix)]
    fn test_create_symlink_directory() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src_dir");
        let dst = temp_dir.path().join("dst_link");

        fs::create_dir(&src).unwrap();
        fs::write(src.join("file.txt"), "content").unwrap();

        create_symlink(&src, &dst);

        assert!(dst.is_symlink());
        assert_eq!(fs::read_link(&dst).unwrap(), src);
        assert!(dst.join("file.txt").exists());
    }

    #[test]
    fn test_process_link_src_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("nonexistent.txt");
        let dst = temp_dir.path().join("dst.txt");
        let symlinks_dir = temp_dir.path().join("symlinks");

        // Should not panic, just skip
        process_link(&src, &dst, &symlinks_dir);

        // No symlink created
        assert!(!dst.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_process_link_creates_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst_link");
        let symlinks_dir = temp_dir.path().join("symlinks");

        fs::write(&src, "source content").unwrap();

        process_link(&src, &dst, &symlinks_dir);

        assert!(dst.is_symlink());
        assert_eq!(fs::read_link(&dst).unwrap(), src);
    }

    #[test]
    #[cfg(unix)]
    fn test_process_link_backs_up_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst.txt");
        let symlinks_dir = temp_dir.path().join("symlinks");

        fs::write(&src, "source content").unwrap();
        fs::write(&dst, "existing content").unwrap();

        process_link(&src, &dst, &symlinks_dir);

        // Destination should now be a symlink
        assert!(dst.is_symlink());

        // Backup should exist
        assert!(symlinks_dir.exists());
        let entries: Vec<_> = fs::read_dir(&symlinks_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    #[cfg(unix)]
    fn test_process_link_correct_symlink_unchanged() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst_link");
        let symlinks_dir = temp_dir.path().join("symlinks");

        fs::write(&src, "source content").unwrap();
        symlink(&src, &dst).unwrap();

        process_link(&src, &dst, &symlinks_dir);

        // Symlink should still be correct
        assert!(dst.is_symlink());
        assert_eq!(fs::read_link(&dst).unwrap(), src);

        // No backup should be created
        assert!(!symlinks_dir.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_process_link_fixes_wrong_symlink() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let wrong_target = temp_dir.path().join("wrong.txt");
        let dst = temp_dir.path().join("dst_link");
        let symlinks_dir = temp_dir.path().join("symlinks");

        fs::write(&src, "correct content").unwrap();
        fs::write(&wrong_target, "wrong content").unwrap();
        symlink(&wrong_target, &dst).unwrap();

        process_link(&src, &dst, &symlinks_dir);

        // Symlink should now point to correct source
        assert!(dst.is_symlink());
        assert_eq!(fs::read_link(&dst).unwrap(), src);
    }

    #[test]
    fn test_link_task_new() {
        let profile = create_test_profile();
        let task = LinkTask::new(profile.clone());
        assert_eq!(task.profile.machine_id, profile.machine_id);
        assert_eq!(task.profile.environment, profile.environment);
    }
}
