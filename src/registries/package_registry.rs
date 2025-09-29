use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::registry::{Registry, RegistryEntryLike};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagerEntry {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub output_file: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub platforms: Option<Vec<String>>,
}

impl RegistryEntryLike for PackageManagerEntry {
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

pub type PackageRegistry = Registry<PackageManagerEntry>;

impl Default for PackageRegistry {
    fn default() -> Self {
        let mut entries = HashMap::new();

        // Homebrew packages
        entries.insert(
            "brew".to_string(),
            PackageManagerEntry {
                name: "Homebrew Packages".to_string(),
                command: "brew".to_string(),
                args: vec!["leaves".to_string()],
                output_file: "brew.txt".to_string(),
                enabled: true,
                description: Some("Homebrew installed packages (leaves only)".to_string()),
                platforms: Some(vec!["macos".to_string(), "linux".to_string()]),
            },
        );

        // Homebrew casks
        entries.insert(
            "brew_cask".to_string(),
            PackageManagerEntry {
                name: "Homebrew Casks".to_string(),
                command: "brew".to_string(),
                args: vec!["list".to_string(), "--cask".to_string()],
                output_file: "brew-cask.txt".to_string(),
                enabled: true,
                description: Some("Homebrew installed casks (applications)".to_string()),
                platforms: Some(vec!["macos".to_string()]),
            },
        );

        // npm global packages
        entries.insert(
            "npm".to_string(),
            PackageManagerEntry {
                name: "npm Global Packages".to_string(),
                command: "npm".to_string(),
                args: vec!["ls".to_string(), "-g".to_string()],
                output_file: "npm.txt".to_string(),
                enabled: true,
                description: Some("npm globally installed packages".to_string()),
                platforms: None,
            },
        );

        // Yarn global packages
        entries.insert(
            "yarn".to_string(),
            PackageManagerEntry {
                name: "Yarn Global Packages".to_string(),
                command: "yarn".to_string(),
                args: vec!["global".to_string(), "list".to_string()],
                output_file: "yarn.txt".to_string(),
                enabled: true,
                description: Some("Yarn globally installed packages".to_string()),
                platforms: None,
            },
        );

        // pnpm global packages
        entries.insert(
            "pnpm".to_string(),
            PackageManagerEntry {
                name: "pnpm Global Packages".to_string(),
                command: "pnpm".to_string(),
                args: vec!["ls".to_string(), "-g".to_string()],
                output_file: "pnpm.txt".to_string(),
                enabled: true,
                description: Some("pnpm globally installed packages".to_string()),
                platforms: None,
            },
        );

        // Bun global packages
        entries.insert(
            "bun".to_string(),
            PackageManagerEntry {
                name: "Bun Global Packages".to_string(),
                command: "bun".to_string(),
                args: vec!["pm".to_string(), "ls".to_string(), "-g".to_string()],
                output_file: "bun.txt".to_string(),
                enabled: true,
                description: Some("Bun globally installed packages".to_string()),
                platforms: None,
            },
        );

        // Cargo packages
        entries.insert(
            "cargo".to_string(),
            PackageManagerEntry {
                name: "Cargo Packages".to_string(),
                command: "cargo".to_string(),
                args: vec!["install".to_string(), "--list".to_string()],
                output_file: "cargo.txt".to_string(),
                enabled: true,
                description: Some("Cargo installed packages".to_string()),
                platforms: None,
            },
        );

        // uv packages
        entries.insert(
            "uv".to_string(),
            PackageManagerEntry {
                name: "uv Packages".to_string(),
                command: "uv".to_string(),
                args: vec!["tool".to_string(), "list".to_string()],
                output_file: "uv.txt".to_string(),
                enabled: true,
                description: Some("uv installed tools".to_string()),
                platforms: None,
            },
        );

        // pip global packages
        entries.insert(
            "pip".to_string(),
            PackageManagerEntry {
                name: "pip Packages".to_string(),
                command: "pip".to_string(),
                args: vec!["list".to_string(), "--format=freeze".to_string()],
                output_file: "pip.txt".to_string(),
                enabled: false,
                description: Some("pip installed packages (system-wide)".to_string()),
                platforms: None,
            },
        );

        Self {
            version: "1.0.0".to_string(),
            entries,
        }
    }
}

impl PackageRegistry {
    pub fn get_platform_compatible_entries<'a>(
        &'a self,
        current_platform: &'a str,
    ) -> impl Iterator<Item = (&String, &PackageManagerEntry)> + 'a {
        self.entries.iter().filter(move |(_, entry)| {
            entry.enabled
                && match &entry.platforms {
                    Some(platforms) => platforms.contains(&current_platform.to_string()),
                    None => true,
                }
        })
    }

    pub fn get_current_platform() -> String {
        #[cfg(target_os = "macos")]
        return "macos".into();
        #[cfg(target_os = "linux")]
        return "linux".into();
        #[cfg(target_os = "windows")]
        return "windows".into();
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        return "unknown".into();
    }
}
