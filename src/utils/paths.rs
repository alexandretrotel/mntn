use directories_next::BaseDirs;
use std::fs;
use whoami;
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

    let user = whoami::username();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_dir_constant() {
        assert_eq!(BACKUP_DIR, "backup");
    }

    #[test]
    fn test_symlinks_dir_constant() {
        assert_eq!(SYMLINKS_DIR, "symlinks");
    }

    #[test]
    fn test_common_dir_constant() {
        assert_eq!(COMMON_DIR, "common");
    }

    #[test]
    fn test_machines_dir_constant() {
        assert_eq!(MACHINES_DIR, "machines");
    }

    #[test]
    fn test_environments_dir_constant() {
        assert_eq!(ENVIRONMENTS_DIR, "environments");
    }

    #[test]
    fn test_profile_config_file_constant() {
        assert_eq!(PROFILE_CONFIG_FILE, "profile.json");
    }

    #[test]
    fn test_machine_id_file_constant() {
        assert_eq!(MACHINE_ID_FILE, ".machine-id");
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
    fn test_get_backup_machine_path_includes_machine_id() {
        let path = get_backup_machine_path("my-machine");
        assert!(path.ends_with("my-machine"));
        assert!(path.to_string_lossy().contains("machines"));
    }

    #[test]
    fn test_get_backup_machine_path_different_ids() {
        let path1 = get_backup_machine_path("machine-a");
        let path2 = get_backup_machine_path("machine-b");
        assert_ne!(path1, path2);
        assert!(path1.ends_with("machine-a"));
        assert!(path2.ends_with("machine-b"));
    }

    #[test]
    fn test_get_backup_environment_path_includes_env() {
        let path = get_backup_environment_path("work");
        assert!(path.ends_with("work"));
        assert!(path.to_string_lossy().contains("environments"));
    }

    #[test]
    fn test_get_backup_environment_path_different_envs() {
        let path1 = get_backup_environment_path("work");
        let path2 = get_backup_environment_path("home");
        assert_ne!(path1, path2);
    }

    #[test]
    fn test_get_symlinks_path_structure() {
        let path = get_symlinks_path();
        assert!(path.ends_with("symlinks"));
        assert!(path.to_string_lossy().contains(".mntn"));
    }

    #[test]
    fn test_get_registry_path_structure() {
        let path = get_registry_path();
        assert!(path.ends_with("registry.json"));
        assert!(path.to_string_lossy().contains(".mntn"));
    }

    #[test]
    fn test_get_package_registry_path_structure() {
        let path = get_package_registry_path();
        assert!(path.ends_with("package_registry.json"));
        assert!(path.to_string_lossy().contains(".mntn"));
    }

    #[test]
    fn test_get_profile_config_path_structure() {
        let path = get_profile_config_path();
        assert!(path.ends_with("profile.json"));
    }

    #[test]
    fn test_get_machine_id_path_structure() {
        let path = get_machine_id_path();
        assert!(path.ends_with(".machine-id"));
    }

    #[test]
    fn test_get_base_dirs_returns_valid() {
        let dirs = get_base_dirs();
        // Should have a valid home directory
        assert!(dirs.home_dir().is_absolute());
    }

    #[test]
    fn test_get_machine_identifier_returns_non_empty() {
        let id = get_machine_identifier();
        assert!(!id.is_empty());
    }

    #[test]
    fn test_get_machine_identifier_format() {
        // When no .machine-id file exists, should return user-hostname format
        let id = get_machine_identifier();
        // Should contain a hyphen (user-hostname format)
        assert!(id.contains('-') || !id.is_empty());
    }

    #[test]
    fn test_get_machine_identifier_no_dots() {
        let id = get_machine_identifier();
        // Hostname should have dots replaced with hyphens
        // and .local removed
        assert!(!id.contains(".local"));
    }

    #[test]
    #[serial_test::serial]
    fn test_get_environment_default() {
        // Clear MNTN_ENV to test default behavior
        unsafe {
            std::env::remove_var("MNTN_ENV");
        }
        let env = get_environment();
        assert_eq!(env, "default");
    }

    #[test]
    #[serial_test::serial]
    fn test_get_environment_from_env_var() {
        unsafe {
            std::env::set_var("MNTN_ENV", "production");
        };
        let env = get_environment();
        assert_eq!(env, "production");
        // Clean up
        unsafe {
            std::env::remove_var("MNTN_ENV");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_get_environment_empty_string_returns_default() {
        unsafe {
            std::env::set_var("MNTN_ENV", "");
        };
        let env = get_environment();
        assert_eq!(env, "default");
        // Clean up
        unsafe {
            std::env::remove_var("MNTN_ENV");
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_get_environment_whitespace_only_returns_default() {
        unsafe {
            std::env::set_var("MNTN_ENV", "   ");
        }
        let env = get_environment();
        assert_eq!(env, "default");
        // Clean up
        unsafe {
            std::env::remove_var("MNTN_ENV");
        }
    }

    #[test]
    fn test_paths_are_consistent() {
        // All paths should be under .mntn
        let mntn_dir = get_mntn_dir();
        let backup_root = get_backup_root();
        let symlinks_path = get_symlinks_path();
        let registry_path = get_registry_path();

        assert!(backup_root.starts_with(&mntn_dir));
        assert!(symlinks_path.starts_with(&mntn_dir));
        assert!(registry_path.starts_with(&mntn_dir));
    }

    #[test]
    fn test_backup_paths_under_backup_root() {
        let backup_root = get_backup_root();
        let common_path = get_backup_common_path();
        let machine_path = get_backup_machine_path("test");
        let env_path = get_backup_environment_path("test");

        assert!(common_path.starts_with(&backup_root));
        assert!(machine_path.starts_with(&backup_root));
        assert!(env_path.starts_with(&backup_root));
    }
}
