use crate::utils::system::sync_directory_contents;
use std::fs;
use std::path::Path;

pub fn restore_configs(backup_path: &Path, target_path: &Path, file_name: &str) -> bool {
    if backup_path.is_dir() {
        return restore_directory(backup_path, target_path, file_name);
    }

    let contents = match fs::read(backup_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read backup file for {}: {}", file_name, e);
            return false;
        }
    };

    if let Some(parent) = target_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        eprintln!("Failed to create directory for {}: {}", file_name, e);
        return false;
    }

    match fs::write(target_path, contents) {
        Ok(()) => {
            println!("Restored {}", file_name);
            true
        }
        Err(e) => {
            eprintln!("Failed to restore {}: {}", file_name, e);
            false
        }
    }
}

fn restore_directory(backup_path: &Path, target_path: &Path, dir_name: &str) -> bool {
    if let Err(e) = fs::create_dir_all(target_path) {
        eprintln!("Failed to create target directory for {}: {}", dir_name, e);
        return false;
    }

    match sync_directory_contents(backup_path, target_path) {
        Ok(()) => {
            println!("Restored directory {}", dir_name);
            true
        }
        Err(e) => {
            eprintln!("Failed to restore directory {}: {}", dir_name, e);
            false
        }
    }
}
