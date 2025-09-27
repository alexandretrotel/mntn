use chrono::Local;
use std::{
    fs::{self},
    io,
    path::Path,
};

use crate::logger::log;

/// Recursively calculates the total size in bytes of the given directory or file path.
///
/// Symlinks are ignored and contribute zero to the total size to avoid cycles.
///
/// Returns `None` if the path metadata cannot be accessed or read.
pub fn calculate_dir_size(path: &Path) -> Option<u64> {
    if path.is_symlink() {
        return Some(0);
    }
    let mut size = 0;
    if path.is_file() {
        size += fs::metadata(path).ok()?.len();
    } else if path.is_dir() {
        let entries = fs::read_dir(path).ok()?;
        for entry in entries {
            let entry = entry.ok()?;
            let entry_path = entry.path();
            if entry_path.is_symlink() {
                continue;
            }
            size += calculate_dir_size(&entry_path).unwrap_or(0);
        }
    }
    Some(size)
}

/// Copies an existing file from the `target` path to the missing `source` path.
///
/// This is used when the user already has a config file in the expected location, but the
/// dotfiles repository does not yet have it tracked. Instead of deleting the file, it is
/// safely copied to the repository.
pub fn copy_file_to_source(target: &Path, source: &Path) -> io::Result<()> {
    log(&format!(
        "Copying existing file {} to missing source {}",
        target.display(),
        source.display()
    ));
    if let Some(parent) = source.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(target, source)?;
    Ok(())
}

/// Copies an existing directory from the `target` path to the missing `source` path.
///
/// This ensures the content is preserved in the user's dotfiles repository
/// if it was not already under source control.
pub fn copy_dir_to_source(target: &Path, source: &Path) -> io::Result<()> {
    log(&format!(
        "Copying existing directory {} to missing source {}",
        target.display(),
        source.display()
    ));

    fs::create_dir_all(source)?;
    copy_dir_recursive(target, source)
}

/// Recursively copies the contents of one directory to another.
///
/// This function does not copy the root directory itself, only its contents.
/// It handles nested directories and files.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Backs up an existing file or directory to a timestamped location inside the backup directory.
///
/// This function is used when the symlink target already exists and is not a symlink.
/// The original content is preserved to prevent data loss.
pub fn backup_existing_target(target: &Path, backup_dir: &Path) -> io::Result<()> {
    let filename = target
        .file_name()
        .and_then(|n| Some(n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "backup".to_string());

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("{filename}_{timestamp}"));

    log(&format!(
        "Backing up existing {} to {}",
        target.display(),
        backup_path.display()
    ));

    fs::rename(target, backup_path)?;
    Ok(())
}
