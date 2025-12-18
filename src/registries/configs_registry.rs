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

use crate::impl_registry_entry_like;

impl_registry_entry_like!(RegistryEntry);

/// Categories for organizing registry entries
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Shell,
    Editor,
    Terminal,
    System,
    Development,
    Application,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryParseError {
    pub invalid_category: String,
}

impl std::fmt::Display for CategoryParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown category: '{}'", self.invalid_category)
    }
}

impl std::error::Error for CategoryParseError {}

impl std::str::FromStr for Category {
    type Err = CategoryParseError;

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
            Err(CategoryParseError {
                invalid_category: s.to_string(),
            })
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
            "zed_settings".to_string(),
            RegistryEntry {
                name: "Zed Settings".to_string(),
                source_path: "zed/settings.json".to_string(),
                target_path: get_xdg_or_default_config_path("zed/settings.json"),
                category: Category::Editor,
                enabled: true,
                description: Some("Zed user settings".to_string()),
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
    pub fn list_by_category(&self) -> HashMap<Category, Vec<(String, RegistryEntry)>> {
        let mut by_category: HashMap<Category, Vec<(String, RegistryEntry)>> = HashMap::new();

        for (id, entry) in &self.entries {
            by_category
                .entry(entry.category)
                .or_default()
                .push((id.clone(), entry.clone()));
        }

        by_category
    }
}

/// Get a config path, checking XDG_CONFIG_HOME first, then falling back to ~/.config
fn get_xdg_or_default_config_path(relative_path: &str) -> PathBuf {
    if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config).join(relative_path);
    }
    get_base_dirs()
        .home_dir()
        .join(".config")
        .join(relative_path)
}

/// Get the path to the ghostty config file, considering XDG and platform conventions
fn get_ghostty_config_path() -> PathBuf {
    if std::env::var_os("XDG_CONFIG_HOME").is_some() {
        return get_xdg_or_default_config_path("ghostty/config");
    }

    #[cfg(target_os = "macos")]
    {
        get_base_dirs()
            .home_dir()
            .join("Library/Application Support/com.mitchellh.ghostty/config")
    }

    #[cfg(not(target_os = "macos"))]
    {
        get_xdg_or_default_config_path("ghostty/config")
    }
}
