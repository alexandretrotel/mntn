use directories_next::BaseDirs;
use std::path::PathBuf;

pub const BACKUP_DIR: &str = "backup";
pub const COMMON_DIR: &str = "common";
pub const ENCRYPTED_DIR: &str = "encrypted";

pub const PROFILES_DIR: &str = "profiles";
pub const PROFILE_CONFIG_FILE: &str = "profiles.json";
pub const ACTIVE_PROFILE_FILE: &str = ".active-profile";

pub fn get_mntn_dir() -> PathBuf {
    let base_dirs = BaseDirs::new().unwrap();
    let home_dir = base_dirs.home_dir();
    home_dir.join(".mntn")
}

pub fn get_backup_path() -> PathBuf {
    get_mntn_dir().join(BACKUP_DIR)
}

pub fn get_common_path() -> PathBuf {
    get_backup_path().join(COMMON_DIR)
}

pub fn get_encrypted_common_path() -> PathBuf {
    get_common_path().join(ENCRYPTED_DIR)
}

pub fn get_profiles_path(profile_name: &str) -> PathBuf {
    get_backup_path().join(PROFILES_DIR).join(profile_name)
}

pub fn get_encrypted_profiles_path(profile_name: &str) -> PathBuf {
    get_profiles_path(profile_name).join(ENCRYPTED_DIR)
}

pub fn get_config_registry_path() -> PathBuf {
    get_mntn_dir().join("config.registry.json")
}

pub fn get_package_registry_path() -> PathBuf {
    get_mntn_dir().join("package.registry.json")
}

pub fn get_encrypted_registry_path() -> PathBuf {
    get_mntn_dir().join("encrypted.registry.json")
}

pub fn get_packages_dir() -> PathBuf {
    get_backup_path().join("packages")
}

pub fn get_profiles_config_path() -> PathBuf {
    get_mntn_dir().join(PROFILE_CONFIG_FILE)
}

pub fn get_active_profile_path() -> PathBuf {
    get_mntn_dir().join(ACTIVE_PROFILE_FILE)
}
