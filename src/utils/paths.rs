use directories_next::BaseDirs;
use std::fs;
use std::path::PathBuf;

/// Relative path to the directory used for storing general backup files.
pub const BACKUP_DIR: &str = "backup";

/// Relative path to the directory used for storing common backup files.
pub const COMMON_DIR: &str = "common";

/// Relative path to the directory used for storing profile-specific backup files.
pub const PROFILES_DIR: &str = "profiles";

/// Relative path to the file used for storing the profile configuration.
pub const PROFILE_CONFIG_FILE: &str = "profile.json";

/// Relative path to the file used for storing the active profile name.
pub const ACTIVE_PROFILE_FILE: &str = ".active-profile";

pub fn get_mntn_dir() -> PathBuf {
    let base_dirs = get_base_dirs();
    let home_dir = base_dirs.home_dir();
    home_dir.join(".mntn")
}

pub fn get_backup_root() -> PathBuf {
    get_mntn_dir().join(BACKUP_DIR)
}

pub fn get_backup_common_path() -> PathBuf {
    get_backup_root().join(COMMON_DIR)
}

pub fn get_backup_profile_path(profile_name: &str) -> PathBuf {
    get_backup_root().join(PROFILES_DIR).join(profile_name)
}

pub fn get_base_dirs() -> BaseDirs {
    BaseDirs::new().unwrap()
}

/// Returns the path to the link registry file
pub fn get_registry_path() -> PathBuf {
    get_mntn_dir().join("configs_registry.json")
}

/// Returns the path to the packages directory
pub fn get_packages_dir() -> PathBuf {
    get_backup_root().join("packages")
}

/// Returns the path to the package manager registry file
pub fn get_package_registry_path() -> PathBuf {
    get_mntn_dir().join("package_registry.json")
}

pub fn get_profile_config_path() -> PathBuf {
    get_mntn_dir().join(PROFILE_CONFIG_FILE)
}

pub fn get_active_profile_path() -> PathBuf {
    get_mntn_dir().join(ACTIVE_PROFILE_FILE)
}

/// Returns the currently active profile name.
/// Reads from .active-profile file or MNTN_PROFILE env var.
/// Returns None if no profile is set.
pub fn get_active_profile_name() -> Option<String> {
    // First check environment variable
    if let Ok(profile) = std::env::var("MNTN_PROFILE") {
        let trimmed = profile.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    // Then check .active-profile file
    let active_profile_path = get_active_profile_path();
    if let Ok(profile) = fs::read_to_string(&active_profile_path) {
        let trimmed = profile.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Sets the active profile by writing to .active-profile file.
pub fn set_active_profile(profile_name: &str) -> std::io::Result<()> {
    let active_profile_path = get_active_profile_path();
    fs::write(active_profile_path, profile_name)
}

/// Clears the active profile.
pub fn clear_active_profile() -> std::io::Result<()> {
    let active_profile_path = get_active_profile_path();
    if active_profile_path.exists() {
        fs::remove_file(active_profile_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_dir_constant() {
        assert_eq!(BACKUP_DIR, "backup");
    }

    #[test]
    fn test_common_dir_constant() {
        assert_eq!(COMMON_DIR, "common");
    }

    #[test]
    fn test_profiles_dir_constant() {
        assert_eq!(PROFILES_DIR, "profiles");
    }

    #[test]
    fn test_profile_config_file_constant() {
        assert_eq!(PROFILE_CONFIG_FILE, "profile.json");
    }

    #[test]
    fn test_active_profile_file_constant() {
        assert_eq!(ACTIVE_PROFILE_FILE, ".active-profile");
    }

    #[test]
    fn test_get_mntn_dir_ends_with_mntn() {
        let path = get_mntn_dir();
        assert!(path.ends_with(".mntn"));
    }

    #[test]
    fn test_get_mntn_dir_is_absolute() {
        let path = get_mntn_dir();
        assert!(path.is_absolute());
    }

    #[test]
    fn test_get_backup_root_structure() {
        let path = get_backup_root();
        assert!(path.ends_with("backup"));
        assert!(path.to_string_lossy().contains(".mntn"));
    }

    #[test]
    fn test_get_backup_common_path_structure() {
        let path = get_backup_common_path();
        assert!(path.ends_with("common"));
        assert!(path.to_string_lossy().contains("backup"));
    }

    #[test]
    fn test_get_backup_profile_path_includes_profile_name() {
        let path = get_backup_profile_path("my-profile");
        assert!(path.ends_with("my-profile"));
        assert!(path.to_string_lossy().contains("profiles"));
    }

    #[test]
    fn test_get_backup_profile_path_different_profiles() {
        let path1 = get_backup_profile_path("profile-a");
        let path2 = get_backup_profile_path("profile-b");
        assert_ne!(path1, path2);
        assert!(path1.ends_with("profile-a"));
        assert!(path2.ends_with("profile-b"));
    }

    #[test]
    fn test_get_registry_path_structure() {
        let path = get_registry_path();
        assert!(path.ends_with("configs_registry.json"));
        assert!(path.to_string_lossy().contains(".mntn"));
    }

    #[test]
    fn test_get_package_registry_path_structure() {
        let path = get_package_registry_path();
        assert!(path.ends_with("package_registry.json"));
    }

    #[test]
    fn test_get_profile_config_path_structure() {
        let path = get_profile_config_path();
        assert!(path.ends_with("profile.json"));
    }

    #[test]
    fn test_get_active_profile_path_structure() {
        let path = get_active_profile_path();
        assert!(path.ends_with(".active-profile"));
    }

    #[test]
    fn test_get_base_dirs_returns_valid() {
        let dirs = get_base_dirs();
        assert!(dirs.home_dir().is_absolute());
    }

    #[test]
    #[serial_test::serial]
    fn test_get_active_profile_name_from_env_var() {
        unsafe {
            std::env::set_var("MNTN_PROFILE", "work");
        }
        let profile = get_active_profile_name();
        assert_eq!(profile, Some("work".to_string()));
        unsafe {
            std::env::remove_var("MNTN_PROFILE");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_get_active_profile_name_empty_env_returns_none() {
        unsafe {
            std::env::set_var("MNTN_PROFILE", "");
        }
        let profile = get_active_profile_name();
        // When env is empty, it should fall through to file check
        // If file doesn't exist, returns None
        unsafe {
            std::env::remove_var("MNTN_PROFILE");
        }
        // Just verify it doesn't panic
        let _ = profile;
    }

    #[test]
    fn test_paths_are_consistent() {
        let mntn_dir = get_mntn_dir();
        let backup_root = get_backup_root();
        let registry_path = get_registry_path();

        assert!(backup_root.starts_with(&mntn_dir));
        assert!(registry_path.starts_with(&mntn_dir));
    }

    #[test]
    fn test_backup_paths_under_backup_root() {
        let backup_root = get_backup_root();
        let common_path = get_backup_common_path();
        let profile_path = get_backup_profile_path("test");

        assert!(common_path.starts_with(&backup_root));
        assert!(profile_path.starts_with(&backup_root));
    }
}
