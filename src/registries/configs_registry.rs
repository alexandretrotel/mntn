use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::registry::{Registry, RegistryEntryLike};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub source_path: String,
    pub target_path: TargetPath,
    pub category: Category,
    pub enabled: bool,
    pub description: Option<String>,
}

impl RegistryEntryLike for RegistryEntry {
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Categories for organizing registry entries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Shell,
    Editor,
    Terminal,
    System,
    Development,
    Application,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Shell => write!(f, "shell"),
            Category::Editor => write!(f, "editor"),
            Category::Terminal => write!(f, "terminal"),
            Category::System => write!(f, "system"),
            Category::Development => write!(f, "development"),
            Category::Application => write!(f, "application"),
        }
    }
}

impl std::str::FromStr for Category {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "shell" => Ok(Category::Shell),
            "editor" => Ok(Category::Editor),
            "terminal" => Ok(Category::Terminal),
            "system" => Ok(Category::System),
            "development" => Ok(Category::Development),
            "application" => Ok(Category::Application),
            _ => Err(format!("Unknown category: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetPath {
    Home(String),
    Config(String),
    Data(String),
    Absolute(String),
}

pub type ConfigsRegistry = Registry<RegistryEntry>;

impl Default for ConfigsRegistry {
    fn default() -> Self {
        let mut entries = HashMap::new();

        // Shell configuration
        entries.insert(
            "zshrc".to_string(),
            RegistryEntry {
                name: "Zsh Configuration".to_string(),
                source_path: ".zshrc".to_string(),
                target_path: TargetPath::Home(".zshrc".to_string()),
                category: Category::Shell,
                enabled: true,
                description: Some("Main Zsh shell configuration file".to_string()),
            },
        );

        entries.insert(
            "vimrc".to_string(),
            RegistryEntry {
                name: "Vim Configuration".to_string(),
                source_path: ".vimrc".to_string(),
                target_path: TargetPath::Home(".vimrc".to_string()),
                category: Category::Editor,
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
                target_path: TargetPath::Home(".config".to_string()),
                category: Category::System,
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
                target_path: TargetPath::Data("Code/User/settings.json".to_string()),
                category: Category::Editor,
                enabled: true,
                description: Some("Visual Studio Code user settings".to_string()),
            },
        );

        entries.insert(
            "vscode_keybindings".to_string(),
            RegistryEntry {
                name: "VSCode Keybindings".to_string(),
                source_path: "vscode/keybindings.json".to_string(),
                target_path: TargetPath::Data("Code/User/keybindings.json".to_string()),
                category: Category::Editor,
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
                target_path: TargetPath::Config("ghostty/config".to_string()),
                category: Category::Terminal,
                enabled: true,
                description: Some("Ghostty terminal emulator configuration".to_string()),
            },
        );

        Self {
            version: "1.0.0".to_string(),
            entries,
        }
    }
}

impl ConfigsRegistry {
    pub fn list_by_category(&self) -> HashMap<Category, Vec<(&String, &RegistryEntry)>> {
        let mut by_category: HashMap<Category, Vec<(&String, &RegistryEntry)>> = HashMap::new();

        for (id, entry) in &self.entries {
            by_category
                .entry(entry.category.clone())
                .or_insert_with(Vec::new)
                .push((id, entry));
        }

        by_category
    }
}
