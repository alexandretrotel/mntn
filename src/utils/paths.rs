use directories_next::BaseDirs;
use std::path::PathBuf;

/// Relative path to the directory used for storing general backup files.
pub const BACKUP_DIR: &str = "backup";

/// This directory is a subdirectory of `BACKUP_DIR` and is used when a file or folder
/// would be replaced by a symlink, allowing safe restoration if needed.
pub const SYMLINKS_DIR: &str = "symlinks";

pub fn get_mntn_dir() -> PathBuf {
    let base_dirs = get_base_dirs();
    let home_dir = base_dirs.home_dir();
    home_dir.join(".mntn")
}

/// Resolves the full path to the general backup directory (`BACKUP_DIR`) inside the user's home.
pub fn get_backup_path() -> PathBuf {
    let mntn_dir = get_mntn_dir();
    mntn_dir.join(BACKUP_DIR)
}

/// Resolves the full path to the symlink-specific backup directory (`SYMLINKS_DIR`) inside the user's home.
pub fn get_symlinks_path() -> PathBuf {
    let mntn_dir = get_mntn_dir();
    mntn_dir.join(SYMLINKS_DIR)
}

pub fn get_base_dirs() -> BaseDirs {
    BaseDirs::new().unwrap()
}

/// Returns the path to the link registry file
pub fn get_registry_path() -> PathBuf {
    get_mntn_dir().join("registry.json")
}
