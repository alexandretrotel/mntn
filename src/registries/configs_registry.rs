use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::{
    registry::{Registry, RegistryEntryLike},
    utils::paths::get_base_dirs,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub source_path: String,
    pub target_path: PathBuf,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
        if s.eq_ignore_ascii_case("shell") {
            Ok(Category::Shell)
        } else if s.eq_ignore_ascii_case("editor") {
            Ok(Category::Editor)
        } else if s.eq_ignore_ascii_case("terminal") {
            Ok(Category::Terminal)
        } else if s.eq_ignore_ascii_case("system") {
            Ok(Category::System)
        } else if s.eq_ignore_ascii_case("development") {
            Ok(Category::Development)
        } else if s.eq_ignore_ascii_case("application") {
            Ok(Category::Application)
        } else {
            Err(format!("Unknown category: {}", s))
        }
    }
}

pub type ConfigsRegistry = Registry<RegistryEntry>;

impl Default for ConfigsRegistry {
    fn default() -> Self {
        let mut entries = HashMap::new();

        let base_dirs = get_base_dirs();
        let home_dir = base_dirs.home_dir();
        let data_dir = base_dirs.data_dir();

        // Shell configuration
        entries.insert(
            "zshrc".to_string(),
            RegistryEntry {
                name: "Zsh Configuration".to_string(),
                source_path: ".zshrc".to_string(),
                target_path: home_dir.join(".zshrc"),
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
                target_path: home_dir.join(".vimrc"),
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
                target_path: home_dir.join(".config"),
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
                target_path: data_dir.join("Code/User/settings.json"),
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
                target_path: data_dir.join("Code/User/keybindings.json"),
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
                target_path: get_ghostty_config_path(),
                category: Category::Terminal,
                enabled: true,
                description: Some("Ghostty terminal emulator configuration".to_string()),
            },
        );

        // Git configuration
        entries.insert(
            "git_config".to_string(),
            RegistryEntry {
                name: "Git Config".to_string(),
                source_path: ".gitconfig".to_string(),
                target_path: home_dir.join(".gitconfig"),
                category: Category::Development,
                enabled: true,
                description: Some("Global Git configuration".to_string()),
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
                .entry(entry.category)
                .or_insert_with(Vec::new)
                .push((id, entry));
        }

        by_category
    }
}

/// Get the path to the ghostty config file, considering XDG and platform conventions
fn get_ghostty_config_path() -> PathBuf {
    let base_dirs = get_base_dirs();

    // Check for XDG_CONFIG_HOME first (cross-platform)
    if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config).join("ghostty/config");
    }

    // Platform-specific defaults
    #[cfg(target_os = "macos")]
    {
        // macOS: prefer Application Support directory
        base_dirs
            .home_dir()
            .join("Library/Application Support/com.mitchellh.ghostty/config")
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Linux/Windows: use ~/.config/ghostty/config
        base_dirs.home_dir().join(".config/ghostty/config")
    }
}
