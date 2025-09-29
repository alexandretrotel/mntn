use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

/// Common interface for registry entries
pub trait RegistryEntryLike {
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
}

/// Generic registry type shared by all registries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry<T> {
    pub version: String,
    pub entries: HashMap<String, T>,
}

impl<T> Registry<T>
where
    T: RegistryEntryLike + Clone + Serialize + for<'a> Deserialize<'a>,
{
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            version: "1.0.0".to_string(),
            entries: HashMap::new(),
        }
    }

    /// Load registry from file, creating default if it doesn't exist
    pub fn load_or_create(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let registry: Registry<T> = serde_json::from_str(&content)?;
            Ok(registry)
        } else {
            let registry = Self::new();
            registry.save(path)?;
            Ok(registry)
        }
    }

    /// Save registry to file
    pub fn save(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get all enabled entries
    pub fn get_enabled_entries(&self) -> impl Iterator<Item = (&String, &T)> {
        self.entries.iter().filter(|(_, e)| e.is_enabled())
    }

    /// Add a new entry
    pub fn add_entry(&mut self, id: String, entry: T) {
        self.entries.insert(id, entry);
    }

    /// Remove an entry
    pub fn remove_entry(&mut self, id: &str) -> Option<T> {
        self.entries.remove(id)
    }

    /// Enable/disable an entry
    pub fn set_entry_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        match self.entries.get_mut(id) {
            Some(entry) => {
                entry.set_enabled(enabled);
                Ok(())
            }
            None => Err(format!("Entry '{}' not found", id)),
        }
    }

    /// Get entry by ID
    pub fn get_entry(&self, id: &str) -> Option<&T> {
        self.entries.get(id)
    }

    /// Update an existing entry
    pub fn update_entry(&mut self, id: &str, entry: T) -> Result<(), String> {
        match self.entries.get_mut(id) {
            Some(existing_entry) => {
                *existing_entry = entry;
                Ok(())
            }
            None => Err(format!("Entry '{}' not found", id)),
        }
    }
}
