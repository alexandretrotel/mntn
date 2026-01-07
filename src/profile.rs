use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::utils::paths::{
    ENCRYPTED_DIR, get_active_profile_name, get_backup_common_path, get_backup_profile_path,
    get_backup_root, get_profile_config_path,
};

/// A profile definition stored in profile.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileDefinition {
    pub description: Option<String>,
}

/// The profile configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileConfig {
    pub version: String,
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

    pub fn profile_exists(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }

    pub fn list_profiles(&self) -> Vec<&String> {
        let mut names: Vec<_> = self.profiles.keys().collect();
        names.sort();
        names
    }

    pub fn create_profile(&mut self, name: &str, description: Option<String>) {
        self.profiles
            .insert(name.to_string(), ProfileDefinition { description });
    }

    pub fn delete_profile(&mut self, name: &str) -> bool {
        self.profiles.remove(name).is_some()
    }

    pub fn save_default_if_missing() -> io::Result<bool> {
        let path = get_profile_config_path();
        if path.exists() {
            return Ok(false);
        }

        let config = ProfileConfig {
            version: "1.0.0".to_string(),
            profiles: HashMap::new(),
        };
        config.save(&path)?;
        Ok(true)
    }
}

/// The active profile used for operations
#[derive(Debug, Clone)]
pub struct ActiveProfile {
    /// The profile name (None means using common only)
    pub name: Option<String>,
}

impl std::fmt::Display for ActiveProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(f, "profile={}", name),
            None => write!(f, "common (no active profile)"),
        }
    }
}

impl ActiveProfile {
    /// Creates an ActiveProfile with an explicit profile name
    pub fn with_profile(name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
        }
    }

    /// Creates an ActiveProfile without a specific profile (common only)
    pub fn common_only() -> Self {
        Self { name: None }
    }

    /// Resolves the active profile from CLI args or system state
    /// Priority: CLI arg > MNTN_PROFILE env > .active-profile file
    pub fn resolve(cli_profile: Option<&str>) -> Self {
        if let Some(profile) = cli_profile {
            return Self::with_profile(profile);
        }

        if let Some(profile) = get_active_profile_name() {
            return Self::with_profile(&profile);
        }

        Self::common_only()
    }

    /// Returns the backup directory for this profile
    pub fn get_backup_path(&self) -> PathBuf {
        match &self.name {
            Some(name) => get_backup_profile_path(name),
            None => get_backup_common_path(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLayer {
    Common,
    Profile,
    Legacy,
}

impl std::fmt::Display for SourceLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceLayer::Common => write!(f, "common"),
            SourceLayer::Profile => write!(f, "profile"),
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
    /// Resolves the source file from the layered backup structure.
    /// Priority: profile > common > legacy
    pub fn resolve_source(&self, source_path: &str) -> Option<ResolvedSource> {
        let candidates = self.get_candidate_sources(source_path);

        for (path, layer) in candidates {
            if path.exists() {
                return Some(ResolvedSource { path, layer });
            }
        }

        None
    }

    /// Gets all candidate source paths in priority order
    pub fn get_candidate_sources(&self, source_path: &str) -> Vec<(PathBuf, SourceLayer)> {
        let mut candidates = Vec::new();

        // Profile layer (highest priority if set)
        if let Some(profile_name) = &self.name {
            candidates.push((
                get_backup_profile_path(profile_name).join(source_path),
                SourceLayer::Profile,
            ));
        }

        // Common layer
        candidates.push((
            get_backup_common_path().join(source_path),
            SourceLayer::Common,
        ));

        // Legacy layer (lowest priority)
        candidates.push((get_backup_root().join(source_path), SourceLayer::Legacy));

        candidates
    }

    /// Gets all existing sources for a path
    pub fn get_all_resolved_sources(&self, source_path: &str) -> Vec<ResolvedSource> {
        self.get_candidate_sources(source_path)
            .into_iter()
            .filter(|(path, _)| path.exists())
            .map(|(path, layer)| ResolvedSource { path, layer })
            .collect()
    }

    /// Resolves the encrypted source file from the layered backup structure.
    /// Looks in the 'encrypted/' subdirectory of each layer.
    /// Priority: profile > common > legacy
    pub fn resolve_encrypted_source(&self, source_path: &str) -> Option<ResolvedSource> {
        let candidates = self.get_candidate_encrypted_sources(source_path);

        for (path, layer) in candidates {
            if path.exists() {
                return Some(ResolvedSource { path, layer });
            }
        }

        None
    }

    /// Gets all candidate encrypted source paths in priority order.
    /// These are in the 'encrypted/' subdirectory of each layer.
    pub fn get_candidate_encrypted_sources(
        &self,
        source_path: &str,
    ) -> Vec<(PathBuf, SourceLayer)> {
        let mut candidates = Vec::new();

        // Profile layer (highest priority if set)
        if let Some(profile_name) = &self.name {
            candidates.push((
                get_backup_profile_path(profile_name)
                    .join(ENCRYPTED_DIR)
                    .join(source_path),
                SourceLayer::Profile,
            ));
        }

        // Common layer
        candidates.push((
            get_backup_common_path()
                .join(ENCRYPTED_DIR)
                .join(source_path),
            SourceLayer::Common,
        ));

        // Legacy layer (lowest priority)
        candidates.push((
            get_backup_root().join(ENCRYPTED_DIR).join(source_path),
            SourceLayer::Legacy,
        ));

        candidates
    }

    /// Returns the encrypted backup directory for this profile
    pub fn get_encrypted_backup_path(&self) -> PathBuf {
        self.get_backup_path().join(ENCRYPTED_DIR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_profile_definition_default() {
        let def = ProfileDefinition::default();
        assert!(def.description.is_none());
    }

    #[test]
    fn test_profile_definition_with_description() {
        let def = ProfileDefinition {
            description: Some("Work laptop".to_string()),
        };
        assert_eq!(def.description.unwrap(), "Work laptop");
    }

    #[test]
    fn test_profile_config_default() {
        let config = ProfileConfig::default();
        assert_eq!(config.version, "");
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn test_profile_config_load_valid() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("profile.json");

        let config = ProfileConfig {
            version: "1.0.0".to_string(),
            profiles: HashMap::from([(
                "work".to_string(),
                ProfileDefinition {
                    description: Some("Work profile".to_string()),
                },
            )]),
        };

        config.save(&config_path).unwrap();

        let loaded = ProfileConfig::load(&config_path).unwrap();
        assert_eq!(loaded.version, "1.0.0");
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
            profiles: HashMap::new(),
        };

        config.save(&config_path).unwrap();
        assert!(config_path.exists());
    }

    #[test]
    fn test_profile_config_create_and_delete_profile() {
        let mut config = ProfileConfig::default();

        config.create_profile("work", Some("Work profile".to_string()));
        assert!(config.profile_exists("work"));
        assert_eq!(config.list_profiles().len(), 1);

        let deleted = config.delete_profile("work");
        assert!(deleted);
        assert!(!config.profile_exists("work"));
    }

    #[test]
    fn test_profile_config_list_profiles_sorted() {
        let mut config = ProfileConfig::default();
        config.create_profile("zebra", None);
        config.create_profile("alpha", None);
        config.create_profile("middle", None);

        let profiles = config.list_profiles();
        assert_eq!(profiles, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn test_active_profile_with_profile() {
        let profile = ActiveProfile::with_profile("work");
        assert_eq!(profile.name, Some("work".to_string()));
    }

    #[test]
    fn test_active_profile_common_only() {
        let profile = ActiveProfile::common_only();
        assert!(profile.name.is_none());
    }

    #[test]
    fn test_active_profile_display_with_name() {
        let profile = ActiveProfile::with_profile("work");
        let display = format!("{}", profile);
        assert!(display.contains("profile=work"));
    }

    #[test]
    fn test_active_profile_display_without_name() {
        let profile = ActiveProfile::common_only();
        let display = format!("{}", profile);
        assert!(display.contains("common"));
    }

    #[test]
    fn test_active_profile_resolve_with_cli_arg() {
        let profile = ActiveProfile::resolve(Some("cli-profile"));
        assert_eq!(profile.name, Some("cli-profile".to_string()));
    }

    #[test]
    fn test_source_layer_display() {
        assert_eq!(SourceLayer::Common.to_string(), "common");
        assert_eq!(SourceLayer::Profile.to_string(), "profile");
        assert_eq!(SourceLayer::Legacy.to_string(), "legacy");
    }

    #[test]
    fn test_source_layer_equality() {
        assert_eq!(SourceLayer::Common, SourceLayer::Common);
        assert_ne!(SourceLayer::Common, SourceLayer::Profile);
        assert_ne!(SourceLayer::Profile, SourceLayer::Legacy);
    }

    #[test]
    fn test_get_candidate_sources_with_profile() {
        let profile = ActiveProfile::with_profile("test-profile");
        let candidates = profile.get_candidate_sources("config.txt");

        // Should have 3 candidates: profile, common, legacy
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].1, SourceLayer::Profile);
        assert_eq!(candidates[1].1, SourceLayer::Common);
        assert_eq!(candidates[2].1, SourceLayer::Legacy);
    }

    #[test]
    fn test_get_candidate_sources_without_profile() {
        let profile = ActiveProfile::common_only();
        let candidates = profile.get_candidate_sources("config.txt");

        // Should have 2 candidates: common, legacy (no profile layer)
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].1, SourceLayer::Common);
        assert_eq!(candidates[1].1, SourceLayer::Legacy);
    }

    #[test]
    fn test_get_candidate_sources_profile_in_path() {
        let profile = ActiveProfile::with_profile("unique-profile");
        let candidates = profile.get_candidate_sources("config.txt");

        let profile_path = &candidates[0].0;
        assert!(profile_path.to_string_lossy().contains("unique-profile"));
    }

    #[test]
    fn test_resolve_source_returns_none_when_no_files_exist() {
        let profile = ActiveProfile::with_profile("nonexistent-profile");
        let result = profile.resolve_source("definitely_nonexistent_file_12345.txt");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_all_resolved_sources_empty_when_no_files() {
        let profile = ActiveProfile::with_profile("nonexistent-profile");
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
            profiles: HashMap::from([
                (
                    "test".to_string(),
                    ProfileDefinition {
                        description: Some("Test profile".to_string()),
                    },
                ),
                ("empty".to_string(), ProfileDefinition { description: None }),
            ]),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ProfileConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, original.version);
        assert_eq!(deserialized.profiles.len(), original.profiles.len());
    }

    #[test]
    fn test_get_backup_path_with_profile() {
        let profile = ActiveProfile::with_profile("work");
        let path = profile.get_backup_path();
        assert!(path.to_string_lossy().contains("profiles"));
        assert!(path.to_string_lossy().contains("work"));
    }

    #[test]
    fn test_get_backup_path_common_only() {
        let profile = ActiveProfile::common_only();
        let path = profile.get_backup_path();
        assert!(path.to_string_lossy().contains("common"));
    }

    #[test]
    fn test_get_candidate_encrypted_sources_with_profile() {
        let profile = ActiveProfile::with_profile("test-profile");
        let candidates = profile.get_candidate_encrypted_sources("ssh/config.age");

        // Should have 3 candidates: profile, common, legacy
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].1, SourceLayer::Profile);
        assert_eq!(candidates[1].1, SourceLayer::Common);
        assert_eq!(candidates[2].1, SourceLayer::Legacy);

        // All paths should contain 'encrypted'
        for (path, _) in &candidates {
            assert!(path.to_string_lossy().contains("encrypted"));
        }
    }

    #[test]
    fn test_get_candidate_encrypted_sources_without_profile() {
        let profile = ActiveProfile::common_only();
        let candidates = profile.get_candidate_encrypted_sources("ssh/config.age");

        // Should have 2 candidates: common, legacy (no profile layer)
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].1, SourceLayer::Common);
        assert_eq!(candidates[1].1, SourceLayer::Legacy);
    }

    #[test]
    fn test_resolve_encrypted_source_returns_none_when_no_files_exist() {
        let profile = ActiveProfile::with_profile("nonexistent-profile");
        let result = profile.resolve_encrypted_source("definitely_nonexistent_12345.age");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_encrypted_backup_path_with_profile() {
        let profile = ActiveProfile::with_profile("work");
        let path = profile.get_encrypted_backup_path();
        assert!(path.to_string_lossy().contains("profiles"));
        assert!(path.to_string_lossy().contains("work"));
        assert!(path.to_string_lossy().contains("encrypted"));
    }

    #[test]
    fn test_get_encrypted_backup_path_common_only() {
        let profile = ActiveProfile::common_only();
        let path = profile.get_encrypted_backup_path();
        assert!(path.to_string_lossy().contains("common"));
        assert!(path.to_string_lossy().contains("encrypted"));
    }
}
