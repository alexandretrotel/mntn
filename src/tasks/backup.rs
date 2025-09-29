use crate::logger::log;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::registries::package_registry::PackageRegistry;
use crate::utils::paths::{
    get_backup_path, get_base_dirs, get_package_registry_path, get_registry_path,
};
use crate::utils::system::run_cmd;
use std::fs;
use std::path::PathBuf;

/// Runs the full backup process.
///
/// This function:
/// - Ensures the backup directory exists.
/// - Logs and prints start and completion messages.
/// - Collects global package lists from various package managers, saving each to individual text files.
/// - Backs up key configuration files.
pub fn run() {
    let backup_dir = get_backup_path();
    fs::create_dir_all(&backup_dir).unwrap();

    println!("🔁 Backing up packages...");
    log("Starting backup");

    // Backup package managers using registry
    backup_package_managers(&backup_dir);

    // Backup config files using registry
    backup_config_files_from_registry(&backup_dir);

    println!("✅ Backup complete.");
    log("Backup complete");
}

/// Backs up package managers based on the package registry entries
fn backup_package_managers(backup_dir: &PathBuf) {
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

    for (id, entry) in compatible_entries {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let args: Vec<&str> = entry.args.iter().map(|s| s.as_str()).collect();
            run_cmd(&entry.command, &args)
        }));

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
fn backup_config_files_from_registry(backup_dir: &PathBuf) {
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

    let base_dirs = get_base_dirs();
    let backupable_entries = registry.get_backupable_entries();

    if backupable_entries.is_empty() {
        println!("ℹ️ No configuration files found to backup");
        return;
    }

    println!(
        "🔁 Backing up {} configuration files...",
        backupable_entries.len()
    );

    for (id, entry) in backupable_entries {
        let target_path = match entry.target_path.resolve(&base_dirs) {
            Ok(path) => path,
            Err(e) => {
                println!("⚠️ Failed to resolve target path for {}: {}", entry.name, e);
                log(&format!(
                    "Failed to resolve target path for {}: {}",
                    entry.name, e
                ));
                continue;
            }
        };

        let backup_destination = backup_dir.join(&entry.source_path);

        // Ensure parent directory exists
        if let Some(parent) = backup_destination.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
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
        }

        let result = if target_path.is_dir() {
            backup_directory(&target_path, &backup_destination)
        } else {
            backup_file(&target_path, &backup_destination)
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
    if source.is_symlink() {
        if let Ok(target) = fs::read_link(source) {
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
    if source.is_symlink() {
        if let Ok(target) = fs::read_link(source) {
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
    }

    // Create destination directory if it doesn't exist
    fs::create_dir_all(destination)?;

    // Use rsync to copy directory contents
    let output = Command::new("rsync")
        .args(&["-av", "--delete"])
        .arg(format!("{}/", source.display())) // trailing slash for rsync
        .arg(destination)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr.clone())
            .unwrap_or_else(|_| format!("{:?}", output.stderr));

        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("rsync failed: {}", stderr),
        ));
    }

    Ok(())
}
