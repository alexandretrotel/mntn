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

    #[allow(dead_code)]
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
}

#[derive(Debug, Clone)]
pub struct ActiveProfile {
    #[allow(dead_code)]
    pub name: Option<String>,
    pub machine_id: String,
    pub environment: String,
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

    #[test]
    fn test_active_profile_resolution_priority() {
        let profile = ActiveProfile::resolve(None, Some("test-machine"), Some("work"));

        assert_eq!(profile.machine_id, "test-machine");
        assert_eq!(profile.environment, "work");
    }

    #[test]
    fn test_source_layer_display() {
        assert_eq!(SourceLayer::Common.to_string(), "common");
        assert_eq!(SourceLayer::Machine.to_string(), "machine");
        assert_eq!(SourceLayer::Environment.to_string(), "environment");
        assert_eq!(SourceLayer::Legacy.to_string(), "legacy");
    }
}
