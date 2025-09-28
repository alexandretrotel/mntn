use crate::logger::log;
use crate::registry::LinkRegistry;
use crate::utils::paths::{get_backup_path, get_base_dirs, get_registry_path};
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

    // List of package managers and their backup files + commands
    let files: Vec<(
        &str,
        Box<dyn Fn() -> Result<String, Box<dyn std::error::Error>>>,
    )> = vec![
        ("brew.txt", Box::new(|| run_cmd("brew", &["leaves"]))),
        (
            "brew-cask.txt",
            Box::new(|| run_cmd("brew", &["list", "--cask"])),
        ),
        ("npm.txt", Box::new(|| run_cmd("npm", &["ls", "-g"]))),
        (
            "yarn.txt",
            Box::new(|| run_cmd("yarn", &["global", "list"])),
        ),
        ("pnpm.txt", Box::new(|| run_cmd("pnpm", &["ls", "-g"]))),
        ("bun.txt", Box::new(|| run_cmd("bun", &["pm", "ls", "-g"]))),
        (
            "cargo.txt",
            Box::new(|| run_cmd("cargo", &["install", "--list"])),
        ),
    ];

    // Execute each command and write output to corresponding file
    for (name, cmd_fn) in files {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cmd_fn()));

        match result {
            Ok(Ok(content)) => {
                if let Err(e) = fs::write(backup_dir.join(&name), content) {
                    eprintln!("Failed to write {}: {}", name, e);
                    log(&format!("Failed to write {}: {}", name, e));
                }
            }
            Ok(Err(e)) => {
                eprintln!("Command for {} failed: {}", name, e);
                log(&format!("Command for {} failed: {}", name, e));
                let _ = fs::write(backup_dir.join(&name), "");
            }
            Err(_) => {
                eprintln!("Command for {} panicked", name);
                log(&format!("Command for {} panicked", name));
                let _ = fs::write(backup_dir.join(&name), "");
            }
        }
    }

    // Backup config files using registry
    backup_config_files_from_registry(&backup_dir);

    println!("✅ Backup complete.");
    log("Backup complete");
}

/// Backs up configuration files based on the registry entries
fn backup_config_files_from_registry(backup_dir: &PathBuf) {
    // Load the registry
    let registry_path = get_registry_path();
    let registry = match LinkRegistry::load_or_create(&registry_path) {
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
