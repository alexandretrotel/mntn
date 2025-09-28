use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a single entry in the registry that defines what should be backed up and linked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Human-readable name/description for this entry
    pub name: String,
    /// Source path within the backup directory (relative to ~/.mntn/backup/)
    pub source_path: String,
    /// Target path where the file/folder should be linked (supports environment variables)
    pub target_path: String,
    /// Category for organization (e.g., "shell", "editor", "terminal", "system")
    pub category: String,
    /// Whether this entry is enabled (allows temporarily disabling entries)
    pub enabled: bool,
    /// Optional description
    pub description: Option<String>,
}

/// Registry containing all files and folders that should be backed up and linked
#[derive(Debug, Serialize, Deserialize)]
pub struct LinkRegistry {
    /// Version of the registry format
    pub version: String,
    /// Map of entry IDs to registry entries
    pub entries: HashMap<String, RegistryEntry>,
}

impl Default for LinkRegistry {
    fn default() -> Self {
        let mut entries = HashMap::new();

        // Shell configuration
        entries.insert(
            "zshrc".to_string(),
            RegistryEntry {
                name: "Zsh Configuration".to_string(),
                source_path: ".zshrc".to_string(),
                target_path: "${HOME}/.zshrc".to_string(),
                category: "shell".to_string(),
                enabled: true,
                description: Some("Main Zsh shell configuration file".to_string()),
            },
        );

        entries.insert(
            "vimrc".to_string(),
            RegistryEntry {
                name: "Vim Configuration".to_string(),
                source_path: ".vimrc".to_string(),
                target_path: "${HOME}/.vimrc".to_string(),
                category: "editor".to_string(),
                enabled: true,
                description: Some("Vim editor configuration".to_string()),
            },
        );

        // Configuration directory
        entries.insert(
            "config".to_string(),
            RegistryEntry {
                name: "General Config Directory".to_string(),
                source_path: "config".to_string(),
                target_path: "${HOME}/.config".to_string(),
                category: "system".to_string(),
                enabled: true,
                description: Some(
                    "General configuration directory for various applications".to_string(),
                ),
            },
        );

        // VSCode configuration
        entries.insert(
            "vscode_settings".to_string(),
            RegistryEntry {
                name: "VSCode Settings".to_string(),
                source_path: "vscode/settings.json".to_string(),
                target_path: "${DATA_DIR}/Code/User/settings.json".to_string(),
                category: "editor".to_string(),
                enabled: true,
                description: Some("Visual Studio Code user settings".to_string()),
            },
        );

        entries.insert(
            "vscode_keybindings".to_string(),
            RegistryEntry {
                name: "VSCode Keybindings".to_string(),
                source_path: "vscode/keybindings.json".to_string(),
                target_path: "${DATA_DIR}/Code/User/keybindings.json".to_string(),
                category: "editor".to_string(),
                enabled: true,
                description: Some("Visual Studio Code keybindings".to_string()),
            },
        );

        // Terminal configuration
        entries.insert(
            "ghostty_config".to_string(),
            RegistryEntry {
                name: "Ghostty Terminal Config".to_string(),
                source_path: "ghostty/config".to_string(),
                target_path: "${CONFIG_DIR}/ghostty/config".to_string(),
                category: "terminal".to_string(),
                enabled: true,
                description: Some("Ghostty terminal emulator configuration".to_string()),
            },
        );

        // Self-reference for mntn configuration
        entries.insert(
            "mntn_config".to_string(),
            RegistryEntry {
                name: "mntn Configuration".to_string(),
                source_path: ".mntn".to_string(),
                target_path: "${HOME}/.mntn".to_string(),
                category: "system".to_string(),
                enabled: true,
                description: Some("mntn tool for system maintenance".to_string()),
            },
        );

        Self {
            version: "1.0.0".to_string(),
            entries,
        }
    }
}

impl LinkRegistry {
    /// Load registry from file, creating default if it doesn't exist
    pub fn load_or_create(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let registry: LinkRegistry = serde_json::from_str(&content)?;
            Ok(registry)
        } else {
            let registry = Self::default();
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
    pub fn get_enabled_entries(&self) -> impl Iterator<Item = (&String, &RegistryEntry)> {
        self.entries.iter().filter(|(_, entry)| entry.enabled)
    }

    /// Get entries by category
    pub fn _get_entries_by_category(
        &self,
        category: &str,
    ) -> impl Iterator<Item = (&String, &RegistryEntry)> {
        self.entries
            .iter()
            .filter(move |(_, entry)| entry.category == category)
    }

    /// Add a new entry
    pub fn add_entry(&mut self, id: String, entry: RegistryEntry) {
        self.entries.insert(id, entry);
    }

    /// Remove an entry
    pub fn remove_entry(&mut self, id: &str) -> Option<RegistryEntry> {
        self.entries.remove(id)
    }

    /// Enable/disable an entry
    pub fn set_entry_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        match self.entries.get_mut(id) {
            Some(entry) => {
                entry.enabled = enabled;
                Ok(())
            }
            None => Err(format!("Entry '{}' not found", id)),
        }
    }

    /// Get entry by ID
    pub fn get_entry(&self, id: &str) -> Option<&RegistryEntry> {
        self.entries.get(id)
    }

    /// Update an existing entry
    pub fn _update_entry(&mut self, id: &str, entry: RegistryEntry) -> Result<(), String> {
        match self.entries.get_mut(id) {
            Some(existing_entry) => {
                *existing_entry = entry;
                Ok(())
            }
            None => Err(format!("Entry '{}' not found", id)),
        }
    }

    /// List all entries grouped by category
    pub fn list_by_category(&self) -> HashMap<String, Vec<(&String, &RegistryEntry)>> {
        let mut by_category: HashMap<String, Vec<(&String, &RegistryEntry)>> = HashMap::new();

        for (id, entry) in &self.entries {
            by_category
                .entry(entry.category.clone())
                .or_insert_with(Vec::new)
                .push((id, entry));
        }

        by_category
    }

    /// Get entries that can be backed up (have existing target paths)
    pub fn _get_backupable_entries(&self) -> Vec<(&String, &RegistryEntry)> {
        self.get_enabled_entries()
            .filter(|(_, entry)| {
                // Check if the target path exists for backup
                match expand_path_variables(&entry.target_path) {
                    Ok(target_path) => target_path.exists(),
                    Err(_) => false,
                }
            })
            .collect()
    }
}

/// Expands environment variables in paths
pub fn expand_path_variables(path: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    use crate::utils::paths::get_base_dirs;
    use shellexpand;

    let base_dirs = get_base_dirs();

    // Replace custom variables with actual paths
    let config_dir = base_dirs.config_dir();
    let expanded = path
        .replace("${DATA_DIR}", &base_dirs.data_dir().to_string_lossy())
        .replace("${CONFIG_DIR}", &config_dir.to_string_lossy())
        .replace("${HOME}", &base_dirs.home_dir().to_string_lossy());

    // Use shellexpand to handle any remaining shell variables
    let expanded = shellexpand::full(&expanded)?;

    Ok(PathBuf::from(expanded.as_ref()))
}
