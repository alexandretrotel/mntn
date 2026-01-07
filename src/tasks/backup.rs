use crate::encryption::{encrypt_file, get_encrypted_path, prompt_password};
use crate::logger::{log, log_error, log_info, log_success, log_warning};
use crate::profile::ActiveProfile;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::registries::encrypted_configs_registry::EncryptedConfigsRegistry;
use crate::registries::package_registry::PackageRegistry;
use crate::tasks::core::{PlannedOperation, Task};
use crate::utils::paths::{
    get_encrypted_registry_path, get_package_registry_path, get_packages_dir, get_registry_path,
};
use crate::utils::system::{rsync_directory, run_cmd};
use rayon::prelude::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct BackupTask {
    profile: ActiveProfile,
    skip_encrypted: bool,
}

impl BackupTask {
    pub fn new(profile: ActiveProfile, skip_encrypted: bool) -> Self {
        Self {
            profile,
            skip_encrypted,
        }
    }
}

impl Task for BackupTask {
    fn name(&self) -> &str {
        "Backup"
    }

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let backup_dir = self.profile.get_backup_path();
        fs::create_dir_all(&backup_dir)?;

        println!("🔁 Backing up...");
        println!("   Target: {}", self.profile);

        let package_managers_dir = get_packages_dir();
        fs::create_dir_all(&package_managers_dir)?;

        backup_package_managers(&package_managers_dir);
        backup_config_files(&backup_dir);

        // Handle encrypted configs backup
        if !self.skip_encrypted {
            let encrypted_backup_dir = self.profile.get_encrypted_backup_path();
            fs::create_dir_all(&encrypted_backup_dir)?;

            match prompt_password(true) {
                Ok(password) => {
                    backup_encrypted_config_files(&encrypted_backup_dir, &password);
                }
                Err(e) => {
                    log_warning(&format!("Skipping encrypted backup: {}", e));
                }
            }
        }

        log_success("Backup complete");
        Ok(())
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let backup_dir = self.profile.get_backup_path();
        let package_managers_dir = get_packages_dir();

        if let Ok(registry) = PackageRegistry::load_or_create(&get_package_registry_path()) {
            let current_platform = PackageRegistry::get_current_platform();
            for (_id, entry) in registry.get_platform_compatible_entries(&current_platform) {
                operations.push(PlannedOperation::with_target(
                    format!("Backup {} package list", entry.name),
                    package_managers_dir
                        .join(&entry.output_file)
                        .display()
                        .to_string(),
                ));
            }
        }

        if let Ok(registry) = ConfigsRegistry::load_or_create(&get_registry_path()) {
            for (_id, entry) in registry.get_enabled_entries() {
                operations.push(PlannedOperation::with_target(
                    format!("Backup {}", entry.name),
                    backup_dir.join(&entry.source_path).display().to_string(),
                ));
            }
        }

        // Include encrypted configs in dry-run if not skipped
        if !self.skip_encrypted {
            let encrypted_backup_dir = self.profile.get_encrypted_backup_path();
            if let Ok(registry) =
                EncryptedConfigsRegistry::load_or_create(&get_encrypted_registry_path())
            {
                for (_id, entry) in registry.get_enabled_entries() {
                    let encrypted_path =
                        get_encrypted_path(&entry.source_path, entry.encrypt_filename);
                    operations.push(PlannedOperation::with_target(
                        format!("Backup {} (encrypted)", entry.name),
                        encrypted_backup_dir
                            .join(&encrypted_path)
                            .display()
                            .to_string(),
                    ));
                }
            }
        }

        operations
    }
}

pub fn run_with_args(args: crate::cli::BackupArgs) {
    use crate::tasks::core::TaskExecutor;

    let profile = args.resolve_profile();
    TaskExecutor::run(
        &mut BackupTask::new(profile, args.skip_encrypted),
        args.dry_run,
    );
}

/// Backs up package managers based on the package registry entries
fn backup_package_managers(package_managers_dir: &Path) {
    let package_registry_path = get_package_registry_path();
    let package_registry = match PackageRegistry::load_or_create(&package_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error(
                "Failed to load package registry, skipping package backup",
                e,
            );
            return;
        }
    };

    let current_platform = PackageRegistry::get_current_platform();
    let compatible_entries: Vec<_> = package_registry
        .get_platform_compatible_entries(&current_platform)
        .collect();

    if compatible_entries.is_empty() {
        log_info("No package managers found to backup");
        return;
    }

    println!(
        "🔁 Backing up {} package managers...",
        compatible_entries.len()
    );

    let results: Vec<_> = compatible_entries
        .par_iter()
        .map(|(id, entry)| {
            let args: Vec<&str> = entry.args.iter().map(|s| s.as_str()).collect();
            let result = match run_cmd(&entry.command, &args) {
                Ok(content) => Ok(content),
                Err(e) => Err(e.to_string()),
            };
            ((*id).clone(), (*entry).clone(), result)
        })
        .collect();

    for (id, entry, result) in results {
        match result {
            Ok(content) => {
                let output_path = package_managers_dir.join(&entry.output_file);
                let tmp_path = output_path.with_extension("tmp");

                // Write to a temporary file first
                match fs::File::create(&tmp_path).and_then(|mut f| f.write_all(content.as_bytes()))
                {
                    Ok(_) => {
                        // Atomically rename the temp file to the final destination
                        if let Err(e) = fs::rename(&tmp_path, &output_path) {
                            log_warning(&format!(
                                "Failed to atomically move {}: {}",
                                entry.output_file, e
                            ));
                        } else {
                            println!("🔁 Backed up {} ({})", entry.name, id);
                            log(&format!("Backed up {}", entry.name));
                        }
                    }
                    Err(e) => {
                        log_warning(&format!(
                            "Failed to write temp file for {}: {}",
                            entry.output_file, e
                        ));
                        // Clean up temp file if it exists
                        let _ = fs::remove_file(&tmp_path);
                    }
                }
            }
            Err(e) => {
                log_warning(&format!("Command for {} failed: {}", entry.name, e));
            }
        }
    }
}

/// Backs up configuration files based on the registry entries
fn backup_config_files(backup_dir: &Path) {
    let registry_path = get_registry_path();
    let registry = match ConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error("Failed to load registry, skipping config file backup", e);
            return;
        }
    };

    let enabled_entries: Vec<_> = registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        log_info("No configuration files found to backup");
        return;
    }

    println!(
        "🔁 Backing up {} configuration files...",
        enabled_entries.len()
    );

    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;
        let backup_destination = backup_dir.join(&entry.source_path);

        if let Some(parent) = backup_destination.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            log_warning(&format!(
                "Failed to create backup directory for {}: {}",
                entry.name, e
            ));
            continue;
        }

        let result = if target_path.is_dir() {
            backup_directory(target_path, &backup_destination)
        } else {
            backup_file(target_path, &backup_destination)
        };

        match result {
            Ok(()) => {
                println!("🔁 Backed up {} ({})", entry.name, id);
                log(&format!(
                    "Backed up {} from {}",
                    entry.name,
                    target_path.display()
                ));
            }
            Err(e) => {
                log_warning(&format!("Failed to backup {}: {}", entry.name, e));
            }
        }
    }
}

/// Backs up a single file.
/// If the source is a symlink pointing to our backup location (legacy behavior),
/// converts it to a real file first to support migration from symlink-based system.
fn backup_file(source: &PathBuf, destination: &PathBuf) -> std::io::Result<()> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source file {} not found", source.display()),
        ));
    }

    // Handle symlink migration: if source is a symlink pointing to our backup,
    // read content from backup and replace symlink with real file
    if source.is_symlink()
        && let Ok(link_target) = fs::read_link(source)
    {
        let canonical_target = link_target.canonicalize().unwrap_or(link_target.clone());
        let canonical_dest = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.clone());

        if canonical_target == canonical_dest {
            // Source is symlink to backup - read from backup, replace symlink with real file
            let content = fs::read(&canonical_target)?;
            fs::remove_file(source)?; // Remove symlink
            fs::write(source, &content)?; // Write real file
            log(&format!(
                "Converted symlink to real file: {}",
                source.display()
            ));
            // Skip the redundant copy since the file is already restored from backup
            return Ok(());
        }
    }

    fs::copy(source, destination)?;
    Ok(())
}

/// Backs up a directory using rsync for efficiency.
/// If the source is a symlink pointing to our backup location (legacy behavior),
/// converts it to a real directory first to support migration from symlink-based system.
fn backup_directory(source: &PathBuf, destination: &PathBuf) -> std::io::Result<()> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source directory {} not found", source.display()),
        ));
    }

    // Handle symlink migration: if source is a symlink pointing to our backup,
    // copy content from backup and replace symlink with real directory
    if source.is_symlink()
        && let Ok(link_target) = fs::read_link(source)
    {
        let canonical_target = link_target
            .canonicalize()
            .unwrap_or_else(|_| link_target.clone());
        let canonical_dest = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.clone());

        if canonical_target == canonical_dest
            || canonical_dest.starts_with(&canonical_target)
            || canonical_target.starts_with(&canonical_dest)
        {
            // Source is symlink to backup - copy from backup, replace symlink with real directory
            fs::remove_file(source)?; // Remove symlink
            fs::create_dir_all(source)?;
            crate::utils::filesystem::copy_dir_recursive(&canonical_target, source)?;
            log(&format!(
                "Converted symlink to real directory: {}",
                source.display()
            ));
            // Skip the redundant rsync since the directory is already restored from backup
            return Ok(());
        }
    }

    fs::create_dir_all(destination)?;
    rsync_directory(source, destination)
}

/// Backs up encrypted configuration files based on the encrypted registry entries
fn backup_encrypted_config_files(encrypted_backup_dir: &Path, password: &str) {
    let registry_path = get_encrypted_registry_path();
    let registry = match EncryptedConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error(
                "Failed to load encrypted registry, skipping encrypted backup",
                e,
            );
            return;
        }
    };

    let enabled_entries: Vec<_> = registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        log_info("No encrypted configuration files found to backup");
        return;
    }

    println!(
        "🔐 Backing up {} encrypted configuration files...",
        enabled_entries.len()
    );

    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;

        if !target_path.exists() {
            log_warning(&format!(
                "Source file for {} not found: {}",
                entry.name,
                target_path.display()
            ));
            continue;
        }

        // Get the encrypted destination path
        let encrypted_path = get_encrypted_path(&entry.source_path, entry.encrypt_filename);
        let backup_destination = encrypted_backup_dir.join(&encrypted_path);

        // Ensure parent directory exists
        if let Some(parent) = backup_destination.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            log_warning(&format!(
                "Failed to create backup directory for {}: {}",
                entry.name, e
            ));
            continue;
        }

        // Encrypt and backup the file
        match encrypt_file(target_path, &backup_destination, password) {
            Ok(()) => {
                println!("🔐 Backed up {} ({})", entry.name, id);
                log(&format!(
                    "Backed up encrypted {} from {}",
                    entry.name,
                    target_path.display()
                ));
            }
            Err(e) => {
                log_warning(&format!("Failed to backup encrypted {}: {}", entry.name, e));
            }
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
    fn test_backup_task_name() {
        let task = BackupTask::new(create_test_profile(), true);
        assert_eq!(task.name(), "Backup");
    }

    #[test]
    fn test_backup_task_new() {
        let profile = create_test_profile();
        let task = BackupTask::new(profile.clone(), false);
        assert_eq!(task.profile.name, profile.name);
        assert!(!task.skip_encrypted);
    }

    #[test]
    fn test_backup_task_new_skip_encrypted() {
        let profile = create_test_profile();
        let task = BackupTask::new(profile.clone(), true);
        assert!(task.skip_encrypted);
    }

    #[test]
    fn test_backup_task_dry_run() {
        let task = BackupTask::new(create_test_profile(), true);
        // Should not panic - just verify it returns successfully
        let _ops = task.dry_run();
    }

    #[test]
    fn test_backup_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let destination = temp_dir.path().join("dest.txt");

        fs::write(&source, "source content").unwrap();

        let result = backup_file(&source, &destination);
        assert!(result.is_ok());

        assert!(destination.exists());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "source content");
    }

    #[test]
    fn test_backup_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("nonexistent.txt");
        let destination = temp_dir.path().join("dest.txt");

        let result = backup_file(&source, &destination);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_backup_file_overwrites_destination() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let destination = temp_dir.path().join("dest.txt");

        fs::write(&source, "new content").unwrap();
        fs::write(&destination, "old content").unwrap();

        let result = backup_file(&source, &destination);
        assert!(result.is_ok());

        assert_eq!(fs::read_to_string(&destination).unwrap(), "new content");
    }

    #[test]
    #[cfg(unix)]
    fn test_backup_file_converts_symlink_to_real_file() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let destination = temp_dir.path().join("dest.txt");
        let source = temp_dir.path().join("source_link");

        // Create destination first, then symlink source to it
        fs::write(&destination, "backup content").unwrap();
        symlink(&destination, &source).unwrap();

        let result = backup_file(&source, &destination);
        assert!(result.is_ok());

        // Source should now be a real file, not a symlink
        assert!(!source.is_symlink());
        assert!(source.is_file());
        assert_eq!(fs::read_to_string(&source).unwrap(), "backup content");

        // Destination should remain unchanged
        assert_eq!(fs::read_to_string(&destination).unwrap(), "backup content");
    }

    #[test]
    fn test_backup_directory_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("nonexistent_dir");
        let destination = temp_dir.path().join("dest_dir");

        let result = backup_directory(&source, &destination);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    #[cfg(unix)]
    fn test_backup_directory_success() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source_dir");
        let destination = temp_dir.path().join("dest_dir");

        // Create source directory with content
        fs::create_dir(&source).unwrap();
        fs::write(source.join("file.txt"), "content").unwrap();

        let result = backup_directory(&source, &destination);

        // Skip test if rsync not available
        if result.is_err()
            && result
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("No such file")
        {
            return;
        }

        assert!(result.is_ok());
        assert!(destination.exists());
        assert!(destination.join("file.txt").exists());
    }

    #[test]
    fn test_backup_directory_creates_destination() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source_dir");
        let destination = temp_dir.path().join("nested").join("dest_dir");

        fs::create_dir(&source).unwrap();

        // This will fail without rsync, but should at least create the destination dir
        let _ = backup_directory(&source, &destination);

        // Even if rsync fails, destination parent should be created
        assert!(destination.parent().unwrap().exists());
    }

    #[test]
    fn test_backup_task_with_profile() {
        let task = BackupTask::new(ActiveProfile::with_profile("work"), true);
        let _ops = task.dry_run();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_backup_task_common_only() {
        let task = BackupTask::new(ActiveProfile::common_only(), true);
        let _ops = task.dry_run();
        // Just verify it doesn't panic
    }
}
