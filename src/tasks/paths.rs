use dirs_next::home_dir;
use std::path::PathBuf;

/// Relative path to the directory used for storing general backup files.
///
/// This path is resolved relative to the user's home directory.
pub const BACKUP_DIR: &str = "dotfiles/backups";

/// Relative path to the directory used for storing backups of overwritten symlink targets.
///
/// This directory is a subdirectory of `BACKUP_DIR` and is used when a file or folder
/// would be replaced by a symlink, allowing safe restoration if needed.
pub const SYMLINK_BACKUP_DIR: &str = "dotfiles/backups/symlinks";

/// Resolves the full path to the general backup directory (`BACKUP_DIR`) inside the user's home.
///
/// # Returns
/// A [`PathBuf`] pointing to `$HOME/dotfiles/backups`.
///
/// # Panics
/// Panics if the user's home directory cannot be determined.
///
/// # Example
/// ```
/// let path = get_backup_path();
/// assert!(path.ends_with("dotfiles/backups"));
/// ```
pub fn get_backup_path() -> PathBuf {
    home_dir().unwrap().join(BACKUP_DIR)
}

/// Resolves the full path to the symlink-specific backup directory (`SYMLINK_BACKUP_DIR`) inside the user's home.
///
/// This is used for backing up existing files or directories before they are replaced with symlinks.
///
/// # Returns
/// A [`PathBuf`] pointing to `$HOME/dotfiles/backups/symlinks`.
///
/// # Panics
/// Panics if the user's home directory cannot be determined.
///
/// # Example
/// ```
/// let path = get_symlink_backup_path();
/// assert!(path.ends_with("dotfiles/backups/symlinks"));
/// ```
pub fn get_symlink_backup_path() -> PathBuf {
    home_dir().unwrap().join(SYMLINK_BACKUP_DIR)
}
