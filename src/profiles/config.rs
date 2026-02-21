use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use crate::utils::paths::get_profiles_config_path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileDefinition {
    pub description: Option<String>,
}

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
        let path = get_profiles_config_path();
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
        let path = get_profiles_config_path();
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
