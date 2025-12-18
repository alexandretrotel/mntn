use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::utils::paths::{
    get_backup_common_path, get_backup_environment_path, get_backup_machine_path, get_backup_root,
    get_environment, get_machine_identifier, get_profile_config_path,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileDefinition {
    pub machine_id: Option<String>,
    pub environment: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileConfig {
    pub version: String,
    pub default_profile: Option<String>,
    pub profiles: HashMap<String, ProfileDefinition>,
}

impl ProfileConfig {
    pub fn load(path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn load_or_default() -> Self {
        let path = get_profile_config_path();
        Self::load(&path).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)
    }

    pub fn get_profile(&self, name: &str) -> Option<&ProfileDefinition> {
        self.profiles.get(name)
    }

    pub fn save_default_if_missing() -> io::Result<bool> {
        let path = get_profile_config_path();
        if path.exists() {
            return Ok(false);
        }

        let config = ProfileConfig {
            version: "1.0.0".to_string(),
            default_profile: None,
            profiles: HashMap::new(),
        };
        config.save(&path)?;
        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct ActiveProfile {
    pub name: Option<String>,
    pub machine_id: String,
    pub environment: String,
}

impl std::fmt::Display for ActiveProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(
                f,
                "profile={} (machine={}, env={})",
                name, self.machine_id, self.environment
            ),
            None => write!(f, "machine={}, env={}", self.machine_id, self.environment),
        }
    }
}

impl ActiveProfile {
    pub fn resolve(
        profile_name: Option<&str>,
        cli_machine_id: Option<&str>,
        cli_env: Option<&str>,
    ) -> Self {
        let config = ProfileConfig::load_or_default();

        let profile_def = profile_name
            .and_then(|name| config.get_profile(name).cloned())
            .or_else(|| {
                config
                    .default_profile
                    .as_ref()
                    .and_then(|name| config.get_profile(name).cloned())
            });

        let machine_id = cli_machine_id
            .map(String::from)
            .or_else(|| profile_def.as_ref().and_then(|p| p.machine_id.clone()))
            .unwrap_or_else(get_machine_identifier);

        let environment = cli_env
            .map(String::from)
            .or_else(|| profile_def.as_ref().and_then(|p| p.environment.clone()))
            .unwrap_or_else(get_environment);

        Self {
            name: profile_name.map(String::from),
            machine_id,
            environment,
        }
    }

    pub fn from_defaults() -> Self {
        Self::resolve(None, None, None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLayer {
    Common,
    Machine,
    Environment,
    Legacy,
}

impl std::fmt::Display for SourceLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceLayer::Common => write!(f, "common"),
            SourceLayer::Machine => write!(f, "machine"),
            SourceLayer::Environment => write!(f, "environment"),
            SourceLayer::Legacy => write!(f, "legacy"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedSource {
    pub path: PathBuf,
    pub layer: SourceLayer,
}

impl ActiveProfile {
    pub fn resolve_source(&self, source_path: &str) -> Option<ResolvedSource> {
        let candidates = self.get_candidate_sources(source_path);

        for (path, layer) in candidates {
            if path.exists() {
                return Some(ResolvedSource { path, layer });
            }
        }

        None
    }

    pub fn get_candidate_sources(&self, source_path: &str) -> Vec<(PathBuf, SourceLayer)> {
        vec![
            (
                get_backup_environment_path(&self.environment).join(source_path),
                SourceLayer::Environment,
            ),
            (
                get_backup_machine_path(&self.machine_id).join(source_path),
                SourceLayer::Machine,
            ),
            (
                get_backup_common_path().join(source_path),
                SourceLayer::Common,
            ),
            (get_backup_root().join(source_path), SourceLayer::Legacy),
        ]
    }

    pub fn get_all_resolved_sources(&self, source_path: &str) -> Vec<ResolvedSource> {
        self.get_candidate_sources(source_path)
            .into_iter()
            .filter(|(path, _)| path.exists())
            .map(|(path, layer)| ResolvedSource { path, layer })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_profile_definition_default() {
        let def = ProfileDefinition::default();
        assert!(def.machine_id.is_none());
        assert!(def.environment.is_none());
        assert!(def.description.is_none());
    }

    #[test]
    fn test_profile_definition_with_values() {
        let def = ProfileDefinition {
            machine_id: Some("my-machine".to_string()),
            environment: Some("work".to_string()),
            description: Some("Work laptop".to_string()),
        };
        assert_eq!(def.machine_id.unwrap(), "my-machine");
        assert_eq!(def.environment.unwrap(), "work");
        assert_eq!(def.description.unwrap(), "Work laptop");
    }

    #[test]
    fn test_profile_config_default() {
        let config = ProfileConfig::default();
        assert_eq!(config.version, "");
        assert!(config.default_profile.is_none());
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn test_profile_config_load_valid() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("profile.json");

        let config = ProfileConfig {
            version: "1.0.0".to_string(),
            default_profile: Some("work".to_string()),
            profiles: HashMap::from([(
                "work".to_string(),
                ProfileDefinition {
                    machine_id: Some("work-machine".to_string()),
                    environment: Some("work".to_string()),
                    description: None,
                },
            )]),
        };

        config.save(&config_path).unwrap();

        let loaded = ProfileConfig::load(&config_path).unwrap();
        assert_eq!(loaded.version, "1.0.0");
        assert_eq!(loaded.default_profile, Some("work".to_string()));
        assert!(loaded.profiles.contains_key("work"));
    }

    #[test]
    fn test_profile_config_load_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("profile.json");

        fs::write(&config_path, "{ invalid json }").unwrap();

        let result = ProfileConfig::load(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_config_load_nonexistent_file() {
        let result = ProfileConfig::load(Path::new("/nonexistent/profile.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_config_save_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("profile.json");

        let config = ProfileConfig {
            version: "1.0.0".to_string(),
            default_profile: None,
            profiles: HashMap::new(),
        };

        config.save(&config_path).unwrap();
        assert!(config_path.exists());
    }

    #[test]
    fn test_profile_config_save_writes_valid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("profile.json");

        let config = ProfileConfig {
            version: "2.0.0".to_string(),
            default_profile: Some("test".to_string()),
            profiles: HashMap::new(),
        };

        config.save(&config_path).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("\"version\""));
        assert!(content.contains("2.0.0"));
    }

    #[test]
    fn test_profile_config_get_profile_exists() {
        let mut config = ProfileConfig::default();
        config.profiles.insert(
            "dev".to_string(),
            ProfileDefinition {
                machine_id: Some("dev-machine".to_string()),
                environment: None,
                description: None,
            },
        );

        let profile = config.get_profile("dev");
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().machine_id, Some("dev-machine".to_string()));
    }

    #[test]
    fn test_profile_config_get_profile_not_exists() {
        let config = ProfileConfig::default();
        assert!(config.get_profile("nonexistent").is_none());
    }

    #[test]
    fn test_active_profile_resolution_priority() {
        let profile = ActiveProfile::resolve(None, Some("test-machine"), Some("work"));

        assert_eq!(profile.machine_id, "test-machine");
        assert_eq!(profile.environment, "work");
    }

    #[test]
    fn test_active_profile_resolve_with_cli_overrides() {
        let profile = ActiveProfile::resolve(None, Some("cli-machine"), Some("cli-env"));

        assert_eq!(profile.machine_id, "cli-machine");
        assert_eq!(profile.environment, "cli-env");
        assert!(profile.name.is_none());
    }

    #[test]
    fn test_active_profile_resolve_with_profile_name() {
        let profile = ActiveProfile::resolve(Some("my-profile"), Some("machine"), Some("env"));

        assert_eq!(profile.name, Some("my-profile".to_string()));
    }

    #[test]
    fn test_active_profile_from_defaults() {
        let profile = ActiveProfile::from_defaults();

        // Should have non-empty machine_id and environment
        assert!(!profile.machine_id.is_empty());
        assert!(!profile.environment.is_empty());
        assert!(profile.name.is_none());
    }

    #[test]
    fn test_active_profile_display_with_name() {
        let profile = ActiveProfile {
            name: Some("work".to_string()),
            machine_id: "my-machine".to_string(),
            environment: "production".to_string(),
        };

        let display = format!("{}", profile);
        assert!(display.contains("profile=work"));
        assert!(display.contains("machine=my-machine"));
        assert!(display.contains("env=production"));
    }

    #[test]
    fn test_active_profile_display_without_name() {
        let profile = ActiveProfile {
            name: None,
            machine_id: "my-machine".to_string(),
            environment: "production".to_string(),
        };

        let display = format!("{}", profile);
        assert!(!display.contains("profile="));
        assert!(display.contains("machine=my-machine"));
        assert!(display.contains("env=production"));
    }

    #[test]
    fn test_source_layer_display() {
        assert_eq!(SourceLayer::Common.to_string(), "common");
        assert_eq!(SourceLayer::Machine.to_string(), "machine");
        assert_eq!(SourceLayer::Environment.to_string(), "environment");
        assert_eq!(SourceLayer::Legacy.to_string(), "legacy");
    }

    #[test]
    fn test_source_layer_equality() {
        assert_eq!(SourceLayer::Common, SourceLayer::Common);
        assert_ne!(SourceLayer::Common, SourceLayer::Machine);
        assert_ne!(SourceLayer::Environment, SourceLayer::Legacy);
    }

    #[test]
    fn test_source_layer_clone() {
        let layer = SourceLayer::Machine;
        let cloned = layer;
        assert_eq!(layer, cloned);
    }

    #[test]
    fn test_get_candidate_sources_returns_four_layers() {
        let profile = ActiveProfile {
            name: None,
            machine_id: "test-machine".to_string(),
            environment: "test-env".to_string(),
        };

        let candidates = profile.get_candidate_sources("config.txt");
        assert_eq!(candidates.len(), 4);
    }

    #[test]
    fn test_get_candidate_sources_priority_order() {
        let profile = ActiveProfile {
            name: None,
            machine_id: "test-machine".to_string(),
            environment: "test-env".to_string(),
        };

        let candidates = profile.get_candidate_sources("config.txt");

        // First should be environment (highest priority)
        assert_eq!(candidates[0].1, SourceLayer::Environment);
        // Second should be machine
        assert_eq!(candidates[1].1, SourceLayer::Machine);
        // Third should be common
        assert_eq!(candidates[2].1, SourceLayer::Common);
        // Fourth should be legacy (lowest priority)
        assert_eq!(candidates[3].1, SourceLayer::Legacy);
    }

    #[test]
    fn test_get_candidate_sources_includes_source_path() {
        let profile = ActiveProfile {
            name: None,
            machine_id: "test-machine".to_string(),
            environment: "test-env".to_string(),
        };

        let candidates = profile.get_candidate_sources("my/config/file.txt");

        for (path, _) in &candidates {
            assert!(
                path.to_string_lossy().contains("my/config/file.txt")
                    || path.ends_with("my/config/file.txt")
            );
        }
    }

    #[test]
    fn test_get_candidate_sources_machine_id_in_path() {
        let profile = ActiveProfile {
            name: None,
            machine_id: "unique-machine-id".to_string(),
            environment: "test-env".to_string(),
        };

        let candidates = profile.get_candidate_sources("config.txt");

        // Machine layer path should contain the machine_id
        let machine_path = &candidates[1].0;
        assert!(machine_path.to_string_lossy().contains("unique-machine-id"));
    }

    #[test]
    fn test_get_candidate_sources_environment_in_path() {
        let profile = ActiveProfile {
            name: None,
            machine_id: "test-machine".to_string(),
            environment: "unique-environment".to_string(),
        };

        let candidates = profile.get_candidate_sources("config.txt");

        // Environment layer path should contain the environment
        let env_path = &candidates[0].0;
        assert!(env_path.to_string_lossy().contains("unique-environment"));
    }

    #[test]
    fn test_resolve_source_returns_none_when_no_files_exist() {
        let profile = ActiveProfile {
            name: None,
            machine_id: "nonexistent-machine".to_string(),
            environment: "nonexistent-env".to_string(),
        };

        let result = profile.resolve_source("definitely_nonexistent_file_12345.txt");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_all_resolved_sources_empty_when_no_files() {
        let profile = ActiveProfile {
            name: None,
            machine_id: "nonexistent-machine".to_string(),
            environment: "nonexistent-env".to_string(),
        };

        let sources = profile.get_all_resolved_sources("definitely_nonexistent_12345.txt");
        assert!(sources.is_empty());
    }

    #[test]
    fn test_resolved_source_clone() {
        let source = ResolvedSource {
            path: PathBuf::from("/some/path"),
            layer: SourceLayer::Common,
        };

        let cloned = source.clone();
        assert_eq!(cloned.path, source.path);
        assert_eq!(cloned.layer, source.layer);
    }

    #[test]
    fn test_profile_config_serialization_roundtrip() {
        let original = ProfileConfig {
            version: "1.0.0".to_string(),
            default_profile: Some("test".to_string()),
            profiles: HashMap::from([
                (
                    "test".to_string(),
                    ProfileDefinition {
                        machine_id: Some("machine-1".to_string()),
                        environment: Some("env-1".to_string()),
                        description: Some("Test profile".to_string()),
                    },
                ),
                (
                    "empty".to_string(),
                    ProfileDefinition {
                        machine_id: None,
                        environment: None,
                        description: None,
                    },
                ),
            ]),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ProfileConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, original.version);
        assert_eq!(deserialized.default_profile, original.default_profile);
        assert_eq!(deserialized.profiles.len(), original.profiles.len());
    }
}
