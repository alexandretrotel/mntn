use crate::logger::log;
use crate::tasks::paths::get_backup_path;
use crate::utils::{
    get_cursor_keybindings_path, get_cursor_settings_path, get_ghostty_config_path,
    get_iterm_preferences_path, get_vscode_keybindings_path, get_vscode_settings_path, run_cmd,
};
use std::fs;
use std::path::PathBuf;

fn restore_editor_file(backup_path: &PathBuf, target_path: Option<PathBuf>, file_name: &str) {
    if let Some(target) = target_path {
        if let Ok(contents) = fs::read_to_string(backup_path) {
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
        } else {
            println!("⚠️ Failed to read backup file for {}", file_name);
            log(&format!("Failed to read backup file for {}", file_name));
        }
    } else {
        println!("⚠️ Target path not found for {}", file_name);
        log(&format!("Target path not found for {}", file_name));
    }
}

fn restore_binary_file(backup_path: &PathBuf, target_path: Option<PathBuf>, file_name: &str) {
    if let Some(target) = target_path {
        if let Ok(contents) = fs::read(backup_path) {
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
        } else {
            println!("⚠️ Failed to read backup file for {}", file_name);
            log(&format!("Failed to read backup file for {}", file_name));
        }
    } else {
        println!("⚠️ Target path not found for {}", file_name);
        log(&format!("Target path not found for {}", file_name));
    }
}

fn install_packages_from_backup(
    backup_path: &PathBuf,
    package_manager: &str,
    install_cmd: Vec<&str>,
) {
    if let Ok(contents) = fs::read_to_string(backup_path) {
        if contents.trim().is_empty() {
            println!("ℹ️ No {} packages to restore", package_manager);
            return;
        }

        println!("📦 Restoring {} packages...", package_manager);
        log(&format!("Restoring {} packages", package_manager));

        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match package_manager {
                "pip" | "uv" => {
                    let mut args = install_cmd.to_vec();
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
                    let mut args = install_cmd.to_vec();
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
                    let mut args = install_cmd.to_vec();
                    args.extend(packages);
                    run_cmd(package_manager, &args)
                }
            }));

        match result {
            Ok(_) => println!("✅ {} packages restored", package_manager),
            Err(_) => println!("⚠️ Failed to restore {} packages", package_manager),
        }
    } else {
        println!("⚠️ No backup file found for {}", package_manager);
        log(&format!("No backup file found for {}", package_manager));
    }
}

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
        ("cursor-settings.json", get_cursor_settings_path()),
        ("cursor-keybindings.json", get_cursor_keybindings_path()),
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

    let binary_files = vec![("iterm-preferences.plist", get_iterm_preferences_path())];

    for (backup_name, target_path) in binary_files {
        let backup_path = backup_dir.join(backup_name);
        if backup_path.exists() {
            restore_binary_file(&backup_path, target_path, backup_name);
        } else {
            println!("ℹ️ No backup found for {}", backup_name);
        }
    }

    println!("✅ Restore complete.");
    log("Restore complete");
}
