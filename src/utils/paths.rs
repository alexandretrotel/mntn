use directories_next::BaseDirs;
use std::fs;
use std::path::PathBuf;

/// Relative path to the directory used for storing general backup files.
pub const BACKUP_DIR: &str = "backup";

/// This directory is a subdirectory of `BACKUP_DIR` and is used when a file or folder
/// would be replaced by a symlink, allowing safe restoration if needed.
pub const SYMLINKS_DIR: &str = "symlinks";

/// Relative path to the directory used for storing common backup files.
pub const COMMON_DIR: &str = "common";

/// Relative path to the directory used for storing machine-specific backup files.
pub const MACHINES_DIR: &str = "machines";

/// Relative path to the directory used for storing environment-specific backup files.
pub const ENVIRONMENTS_DIR: &str = "environments";

/// Relative path to the file used for storing the profile configuration.
pub const PROFILE_CONFIG_FILE: &str = "profile.json";

/// Relative path to the file used for storing the machine identifier.
pub const MACHINE_ID_FILE: &str = ".machine-id";

pub fn get_mntn_dir() -> PathBuf {
    let base_dirs = get_base_dirs();
    let home_dir = base_dirs.home_dir();
    home_dir.join(".mntn")
}

#[deprecated(
    note = "Use layered backup paths via get_backup_common_path, get_backup_machine_path, or get_backup_environment_path"
)]
#[allow(dead_code)]
pub fn get_backup_path() -> PathBuf {
    let mntn_dir = get_mntn_dir();
    mntn_dir.join(BACKUP_DIR)
}

pub fn get_backup_root() -> PathBuf {
    get_mntn_dir().join(BACKUP_DIR)
}

pub fn get_backup_common_path() -> PathBuf {
    get_backup_root().join(COMMON_DIR)
}

pub fn get_backup_machine_path(machine_id: &str) -> PathBuf {
    get_backup_root().join(MACHINES_DIR).join(machine_id)
}

pub fn get_backup_environment_path(env: &str) -> PathBuf {
    get_backup_root().join(ENVIRONMENTS_DIR).join(env)
}

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

/// Returns the path to the package manager registry file
pub fn get_package_registry_path() -> PathBuf {
    get_mntn_dir().join("package_registry.json")
}

pub fn get_profile_config_path() -> PathBuf {
    get_mntn_dir().join(PROFILE_CONFIG_FILE)
}

pub fn get_machine_id_path() -> PathBuf {
    get_mntn_dir().join(MACHINE_ID_FILE)
}

pub fn get_machine_identifier() -> String {
    let machine_id_path = get_machine_id_path();
    if let Ok(id) = fs::read_to_string(&machine_id_path) {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let hostname = gethostname::gethostname()
        .to_string_lossy()
        .to_lowercase()
        .replace(".local", "")
        .replace('.', "-");

    format!("{}-{}", user, hostname)
}

pub fn get_environment() -> String {
    if let Ok(env) = std::env::var("MNTN_ENV") {
        let trimmed = env.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    "default".to_string()
}
