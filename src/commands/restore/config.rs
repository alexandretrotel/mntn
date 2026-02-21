use crate::utils::{
    display::{red, short_component},
    system::sync_directory_contents,
};
use std::fs;
use std::path::Path;

pub fn restore_configs(backup_path: &Path, target_path: &Path) -> bool {
    if backup_path.is_dir() {
        return restore_directory(backup_path, target_path);
    }

    let contents = match fs::read(backup_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                red(&format!(
                    "Failed to read backup file {}: {}",
                    short_component(backup_path),
                    e
                ))
            );
            return false;
        }
    };

    if let Some(parent) = target_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        eprintln!(
            "{}",
            red(&format!(
                "Failed to create directory {}: {}",
                short_component(parent),
                e
            ))
        );
        return false;
    }

    match fs::write(target_path, contents) {
        Ok(()) => true,
        Err(e) => {
            eprintln!(
                "{}",
                red(&format!(
                    "Failed to write {}: {}",
                    short_component(target_path),
                    e
                ))
            );
            false
        }
    }
}

fn restore_directory(backup_path: &Path, target_path: &Path) -> bool {
    if let Err(e) = fs::create_dir_all(target_path) {
        eprintln!(
            "{}",
            red(&format!(
                "Failed to create target directory {}: {}",
                short_component(target_path),
                e
            ))
        );
        return false;
    }

    match sync_directory_contents(backup_path, target_path) {
        Ok(()) => true,
        Err(e) => {
            eprintln!(
                "{}",
                red(&format!(
                    "Failed to restore directory {}: {}",
                    short_component(backup_path),
                    e
                ))
            );
            false
        }
    }
}
