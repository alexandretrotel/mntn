use directories_next::BaseDirs;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::registry::{Registry, RegistryEntryLike};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRegistryEntry {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub source_path: String,
    pub target_path: PathBuf,
}

use crate::impl_registry_entry_like;
use crate::utils::paths::{get_ghostty_config_path, get_xdg_or_default_config_path};

impl_registry_entry_like!(ConfigRegistryEntry);

pub type ConfigRegistry = Registry<ConfigRegistryEntry>;

impl Default for ConfigRegistry {
    fn default() -> Self {
        let mut entries = HashMap::new();

        let base_dirs = BaseDirs::new().unwrap();
        let home_dir = base_dirs.home_dir();
        let data_dir = base_dirs.data_dir();

        entries.insert(
            "bashrc".to_string(),
            ConfigRegistryEntry {
                name: "Bash Configuration".to_string(),
                source_path: ".bashrc".to_string(),
                target_path: home_dir.join(".bashrc"),
                enabled: true,
                description: Some("Bash shell configuration file".to_string()),
            },
        );

        entries.insert(
            "zshrc".to_string(),
            ConfigRegistryEntry {
                name: "Zsh Configuration".to_string(),
                source_path: ".zshrc".to_string(),
                target_path: home_dir.join(".zshrc"),
                enabled: true,
                description: Some("Zsh shell configuration file".to_string()),
            },
        );

        entries.insert(
            "vimrc".to_string(),
            ConfigRegistryEntry {
                name: "Vim Configuration".to_string(),
                source_path: ".vimrc".to_string(),
                target_path: home_dir.join(".vimrc"),
                enabled: true,
                description: Some("Vim editor configuration".to_string()),
            },
        );

        entries.insert(
            "vscode_settings".to_string(),
            ConfigRegistryEntry {
                name: "VSCode Settings".to_string(),
                source_path: "vscode/settings.json".to_string(),
                target_path: data_dir.join("Code/User/settings.json"),
                enabled: true,
                description: Some("Visual Studio Code user settings".to_string()),
            },
        );

        entries.insert(
            "vscode_keybindings".to_string(),
            ConfigRegistryEntry {
                name: "VSCode Keybindings".to_string(),
                source_path: "vscode/keybindings.json".to_string(),
                target_path: data_dir.join("Code/User/keybindings.json"),
                enabled: true,
                description: Some("Visual Studio Code keybindings".to_string()),
            },
        );

        entries.insert(
            "zed_settings".to_string(),
            ConfigRegistryEntry {
                name: "Zed Settings".to_string(),
                source_path: "zed/settings.json".to_string(),
                target_path: get_xdg_or_default_config_path("zed/settings.json"),
                enabled: true,
                description: Some("Zed user settings".to_string()),
            },
        );

        entries.insert(
            "ghostty_config".to_string(),
            ConfigRegistryEntry {
                name: "Ghostty Terminal Config".to_string(),
                source_path: "ghostty/config".to_string(),
                target_path: get_ghostty_config_path(),
                enabled: true,
                description: Some("Ghostty terminal emulator configuration".to_string()),
            },
        );

        entries.insert(
            "git_config".to_string(),
            ConfigRegistryEntry {
                name: "Git Config".to_string(),
                source_path: ".gitconfig".to_string(),
                target_path: home_dir.join(".gitconfig"),
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
