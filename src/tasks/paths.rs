use dirs::home_dir;

pub const BACKUP_DIR: &str = "dotfiles/backups";
pub const SYMLINK_BACKUP_DIR: &str = "dotfiles/backups/symlinks";

pub fn get_backup_path() -> std::path::PathBuf {
    home_dir().unwrap().join(BACKUP_DIR)
}

pub fn get_symlink_backup_path() -> std::path::PathBuf {
    home_dir().unwrap().join(SYMLINK_BACKUP_DIR)
}
