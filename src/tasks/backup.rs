use crate::logger::log;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::registries::package_registry::PackageRegistry;
use crate::tasks::core::{PlannedOperation, Task};
use crate::utils::paths::{get_backup_path, get_package_registry_path, get_registry_path};
use crate::utils::system::run_cmd;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Backup task that saves package lists and configuration files
pub struct BackupTask;

impl Task for BackupTask {
    fn name(&self) -> &str {
        "Backup"
    }

    fn execute(&mut self) {
        let backup_dir = get_backup_path();
        fs::create_dir_all(&backup_dir).unwrap();

        println!("🔁 Backing up...");

        // Backup package managers using registry
        backup_package_managers(&backup_dir);

        // Backup config files using registry
        backup_config_files_from_registry(&backup_dir);

        println!("✅ Backup complete.");
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let backup_dir = get_backup_path();

        // Package managers
        if let Ok(registry) = PackageRegistry::load_or_create(&get_package_registry_path()) {
            let current_platform = PackageRegistry::get_current_platform();
            for (_id, entry) in registry.get_platform_compatible_entries(&current_platform) {
                operations.push(PlannedOperation::with_target(
                    format!("Backup {} package list", entry.name),
                    backup_dir.join(&entry.output_file).display().to_string(),
                ));
            }
        }

        // Config files
        if let Ok(registry) = ConfigsRegistry::load_or_create(&get_registry_path()) {
            for (_id, entry) in registry.get_enabled_entries() {
                operations.push(PlannedOperation::with_target(
                    format!("Backup {}", entry.name),
                    backup_dir.join(&entry.source_path).display().to_string(),
                ));
            }
        }

        operations
    }
}

/// Run with CLI args
pub fn run_with_args(args: crate::cli::BackupArgs) {
    use crate::tasks::core::TaskExecutor;
    TaskExecutor::run(&mut BackupTask, args.dry_run);
}

/// Backs up package managers based on the package registry entries
fn backup_package_managers(backup_dir: &Path) {
    // Load the package registry
    let package_registry_path = get_package_registry_path();
    let package_registry = match PackageRegistry::load_or_create(&package_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!(
                "⚠️ Failed to load package registry, skipping package backup: {}",
                e
            );
            log(&format!("Failed to load package registry: {}", e));
            return;
        }
    };

    let current_platform = PackageRegistry::get_current_platform();
    let compatible_entries: Vec<_> = package_registry
        .get_platform_compatible_entries(&current_platform)
        .collect();

    if compatible_entries.is_empty() {
        println!("ℹ️ No package managers found to backup");
        return;
    }

    println!(
        "🔁 Backing up {} package managers...",
        compatible_entries.len()
    );

    // Run package manager commands in parallel
    let results: Vec<_> = compatible_entries
        .par_iter()
        .map(|(id, entry)| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let args: Vec<&str> = entry.args.iter().map(|s| s.as_str()).collect();
                run_cmd(&entry.command, &args)
            }));
            // Convert Result<Result<String, Box<dyn Error>>, _> to use String for error
            // so it can be sent across threads
            let result = match result {
                Ok(Ok(content)) => Ok(Ok(content)),
                Ok(Err(e)) => Ok(Err(e.to_string())),
                Err(_) => Err(()),
            };
            ((*id).clone(), (*entry).clone(), result)
        })
        .collect();

    // Write results sequentially to avoid output interleaving
    for (id, entry, result) in results {
        match result {
            Ok(Ok(content)) => {
                if let Err(e) = fs::write(backup_dir.join(&entry.output_file), content) {
                    println!("⚠️ Failed to write {}: {}", entry.output_file, e);
                    log(&format!("Failed to write {}: {}", entry.output_file, e));
                } else {
                    println!("🔁 Backed up {} ({})", entry.name, id);
                    log(&format!("Backed up {}", entry.name));
                }
            }
            Ok(Err(e)) => {
                println!("⚠️ Command for {} failed: {}", entry.name, e);
                log(&format!("Command for {} failed: {}", entry.name, e));
                let _ = fs::write(backup_dir.join(&entry.output_file), "");
            }
            Err(_) => {
                println!("⚠️ Command for {} panicked", entry.name);
                log(&format!("Command for {} panicked", entry.name));
                let _ = fs::write(backup_dir.join(&entry.output_file), "");
            }
        }
    }
}

/// Backs up configuration files based on the registry entries
fn backup_config_files_from_registry(backup_dir: &Path) {
    // Load the registry
    let registry_path = get_registry_path();
    let registry = match ConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!(
                "⚠️ Failed to load registry, skipping config file backup: {}",
                e
            );
            log(&format!("Failed to load registry: {}", e));
            return;
        }
    };

    let enabled_entries: Vec<_> = registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("ℹ️ No configuration files found to backup");
        return;
    }

    println!(
        "🔁 Backing up {} configuration files...",
        enabled_entries.len()
    );

    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;
        let backup_destination = backup_dir.join(&entry.source_path);

        // Ensure parent directory exists
        if let Some(parent) = backup_destination.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            println!(
                "⚠️ Failed to create backup directory for {}: {}",
                entry.name, e
            );
            log(&format!(
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
            Ok(_) => {
                println!("🔁 Backed up {} ({})", entry.name, id);
                log(&format!(
                    "Backed up {} from {}",
                    entry.name,
                    target_path.display()
                ));
            }
            Err(e) => {
                println!("⚠️ Failed to backup {}: {}", entry.name, e);
                log(&format!("Failed to backup {}: {}", entry.name, e));
            }
        }
    }
}

/// Backs up a single file
fn backup_file(source: &PathBuf, destination: &PathBuf) -> std::io::Result<()> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source file {} not found", source.display()),
        ));
    }

    // Skip backup if source is a symlink pointing to our backup file
    if source.is_symlink()
        && let Ok(target) = fs::read_link(source)
    {
        let canonical_target = target.canonicalize().unwrap_or(target);
        let canonical_dest = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.clone());

        if canonical_target == canonical_dest {
            log(&format!(
                "Skipping backup of {} - it's already a symlink to our backup location",
                source.display()
            ));
            return Ok(());
        }
    }

    fs::copy(source, destination)?;
    Ok(())
}

/// Backs up a directory using rsync for efficiency
fn backup_directory(source: &PathBuf, destination: &PathBuf) -> std::io::Result<()> {
    use std::process::Command;

    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source directory {} not found", source.display()),
        ));
    }

    // Skip backup if source is a symlink pointing to our backup directory or vice versa
    if source.is_symlink()
        && let Ok(target) = fs::read_link(source)
    {
        // Canonicalize paths to handle relative vs absolute path differences
        let canonical_target = target.canonicalize().unwrap_or_else(|_| target.clone());
        let canonical_dest = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.clone());

        // Check if symlink points to destination or destination is within target
        if canonical_target == canonical_dest
            || canonical_dest.starts_with(&canonical_target)
            || canonical_target.starts_with(&canonical_dest)
        {
            log(&format!(
                "Skipping backup of {} - it's a symlink to/from our backup location (target: {})",
                source.display(),
                target.display()
            ));
            return Ok(());
        }
    }

    // Create destination directory if it doesn't exist
    fs::create_dir_all(destination)?;

    // Use rsync to copy directory contents
    let output = Command::new("rsync")
        .args(["-av", "--delete"])
        .arg(format!("{}/", source.display())) // trailing slash for rsync
        .arg(destination)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr.clone())
            .unwrap_or_else(|_| format!("{:?}", output.stderr));

        return Err(std::io::Error::other(format!("rsync failed: {}", stderr)));
    }

    Ok(())
}
