use crate::logger::log;
use crate::utils::app_paths::{
    get_ghostty_config_path, get_vscode_keybindings_path, get_vscode_settings_path,
};
use crate::utils::paths::get_backup_path;
use std::fs;
use std::path::PathBuf;

/// Main function to run the restore process.
pub fn run() {
    let backup_dir = get_backup_path();

    if !backup_dir.exists() {
        println!("❌ Backup directory not found. Run backup first.");
        log("Backup directory not found");
        return;
    }

    println!("🔄 Starting restore process...");
    log("Starting restore");

    let config_files = vec![
        ("vscode-settings.json", get_vscode_settings_path()),
        ("vscode-keybindings.json", get_vscode_keybindings_path()),
        ("ghostty-config", get_ghostty_config_path()),
    ];

    for (backup_name, target_path) in config_files {
        let backup_path = backup_dir.join(backup_name);
        if backup_path.exists() {
            restore_config_file(&backup_path, target_path, backup_name);
        } else {
            println!("ℹ️ No backup found for {}", backup_name);
        }
    }

    println!("✅ Restore complete.");
    log("Restore complete");
}

/// Attempts to restore a configuration file from a backup to its target location.
///
/// If the backup file exists and the target path is specified, this function:
/// - Reads the contents of the backup file.
/// - Creates parent directories for the target path if they don't exist.
/// - Writes the contents to the target path.
fn restore_config_file(backup_path: &PathBuf, target_path: Option<PathBuf>, file_name: &str) {
    let target = match target_path {
        Some(path) => path,
        None => {
            println!("⚠️ Target path not found for {}", file_name);
            log(&format!("Target path not found for {}", file_name));
            return;
        }
    };

    let contents = match fs::read_to_string(backup_path) {
        Ok(c) => c,
        Err(e) => {
            println!("⚠️ Failed to read backup file for {}: {}", file_name, e);
            log(&format!(
                "Failed to read backup file for {}: {}",
                file_name, e
            ));
            return;
        }
    };

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
