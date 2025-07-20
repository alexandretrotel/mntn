use crate::logger::log;
use crate::tasks::paths::get_backup_path;
use crate::utils::{
    get_ghostty_config_path, get_vscode_keybindings_path, get_vscode_settings_path, run_cmd,
};
use std::fs;
use std::path::PathBuf;

/// Attempts to restore a configuration file from a backup to its target location.
///
/// If the backup file exists and the target path is specified, this function:
/// - Reads the contents of the backup file.
/// - Creates parent directories for the target path if they don't exist.
/// - Writes the contents to the target path.
///
/// Logs and prints messages for success or failure.
///
/// # Arguments
/// * `backup_path` - Path to the backup file to restore from.
/// * `target_path` - Optional path where the file should be restored.
/// * `file_name` - Human-readable name of the file for logging purposes.
fn restore_editor_file(backup_path: &PathBuf, target_path: Option<PathBuf>, file_name: &str) {
    if let Some(target) = target_path {
        match fs::read_to_string(backup_path) {
            Ok(contents) => {
                if let Some(parent) = target.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        println!("⚠️ Failed to create directory for {}: {}", file_name, e);
                        log(&format!(
                            "Failed to create directory for {}: {}",
                            file_name, e
                        ));
                        return;
                    }
                }

                match fs::write(&target, contents) {
                    Ok(_) => {
                        println!("✅ Restored {}", file_name);
                        log(&format!("Restored {}", file_name));
                    }
                    Err(e) => {
                        println!("⚠️ Failed to restore {}: {}", file_name, e);
                        log(&format!("Failed to restore {}: {}", file_name, e));
                    }
                }
            }
            Err(_) => {
                println!("⚠️ Failed to read backup file for {}", file_name);
                log(&format!("Failed to read backup file for {}", file_name));
            }
        }
    } else {
        println!("⚠️ Target path not found for {}", file_name);
        log(&format!("Target path not found for {}", file_name));
    }
}

/// Installs packages from a backup file using the specified package manager.
///
/// Reads the backup file line-by-line and passes packages to the package manager's
/// install command. Supports special cases for `pip`, `uv`, and `brew`.
///
/// # Arguments
/// * `backup_path` - Path to the backup file listing packages.
/// * `package_manager` - Name of the package manager (e.g., "npm", "pip").
/// * `install_cmd` - Command-line arguments to use for installation.
///
/// # Behavior
/// - Skips install if backup is empty.
/// - Runs the package manager command via `run_cmd`.
/// - Catches panics during installation to avoid crashing.
///
/// Logs results and prints progress.
fn install_packages_from_backup(
    backup_path: &PathBuf,
    package_manager: &str,
    install_cmd: Vec<&str>,
) {
    match fs::read_to_string(backup_path) {
        Ok(contents) => {
            if contents.trim().is_empty() {
                println!("ℹ️ No {} packages to restore", package_manager);
                return;
            }

            println!("📦 Restoring {} packages...", package_manager);
            log(&format!("Restoring {} packages", package_manager));

            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match package_manager {
                    "pip" | "uv" => {
                        let mut args = install_cmd.clone();
                        args.push(backup_path.to_str().unwrap());
                        run_cmd(package_manager, &args)
                    }
                    "brew" => {
                        let packages: Vec<&str> = contents
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .collect();
                        if packages.is_empty() {
                            return String::new();
                        }
                        let mut args = install_cmd.clone();
                        args.extend(packages);
                        run_cmd(package_manager, &args)
                    }
                    _ => {
                        let packages: Vec<&str> = contents
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .collect();
                        if packages.is_empty() {
                            return String::new();
                        }
                        let mut args = install_cmd.clone();
                        args.extend(packages);
                        run_cmd(package_manager, &args)
                    }
                }));

            match result {
                Ok(_) => println!("✅ {} packages restored", package_manager),
                Err(_) => println!("⚠️ Failed to restore {} packages", package_manager),
            }
        }
        Err(_) => {
            println!("⚠️ No backup file found for {}", package_manager);
            log(&format!("No backup file found for {}", package_manager));
        }
    }
}

/// Entry point to the restore process.
///
/// - Verifies the backup directory exists.
/// - Restores packages from multiple package manager backup files.
/// - Restores editor configuration files.
///
/// Logs progress and errors, and prints user-friendly messages.
///
/// # Behavior
/// - Checks for the existence of the backup directory.
/// - Iterates over known package managers and their backup files.
/// - Calls `install_packages_from_backup` for each package manager.
/// - Calls `restore_editor_file` for each editor configuration file.
/// - Prints summary messages for success or failure.
pub fn run() {
    let backup_dir = get_backup_path();

    if !backup_dir.exists() {
        println!("❌ Backup directory not found. Run backup first.");
        log("Backup directory not found");
        return;
    }

    println!("🔄 Starting restore process...");
    log("Starting restore");

    let package_managers = vec![
        ("bun.txt", "bun", vec!["install", "-g"]),
        ("npm.txt", "npm", vec!["install", "-g"]),
        ("pnpm.txt", "pnpm", vec!["add", "-g"]),
        ("yarn.txt", "yarn", vec!["global", "add"]),
        ("pip.txt", "pip", vec!["install", "-r"]),
        ("pipx.txt", "pipx", vec!["install"]),
        ("gem.txt", "gem", vec!["install"]),
        ("composer.txt", "composer", vec!["global", "require"]),
        ("uv.txt", "uv", vec!["pip", "install", "-r"]),
        ("brew.txt", "brew", vec!["install"]),
        ("brew-cask.txt", "brew", vec!["install", "--cask"]),
        ("cargo.txt", "cargo", vec!["install"]),
        ("go.txt", "go", vec!["install"]),
    ];

    for (backup_file, manager, install_cmd) in package_managers {
        let backup_path = backup_dir.join(backup_file);
        if backup_path.exists() {
            install_packages_from_backup(&backup_path, manager, install_cmd);
        } else {
            println!("ℹ️ No backup found for {}", manager);
        }
    }

    let editor_files = vec![
        ("vscode-settings.json", get_vscode_settings_path()),
        ("vscode-keybindings.json", get_vscode_keybindings_path()),
        ("ghostty-config", get_ghostty_config_path()),
    ];

    for (backup_name, target_path) in editor_files {
        let backup_path = backup_dir.join(backup_name);
        if backup_path.exists() {
            restore_editor_file(&backup_path, target_path, backup_name);
        } else {
            println!("ℹ️ No backup found for {}", backup_name);
        }
    }

    println!("✅ Restore complete.");
    log("Restore complete");
}
