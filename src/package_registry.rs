use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a package manager that can be backed up
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagerEntry {
    /// Human-readable name for this package manager
    pub name: String,
    /// Command to execute (e.g., "brew")
    pub command: String,
    /// Arguments to pass to the command (e.g., ["leaves"])
    pub args: Vec<String>,
    /// Output filename (e.g., "brew.txt")
    pub output_file: String,
    /// Whether this entry is enabled
    pub enabled: bool,
    /// Optional description
    pub description: Option<String>,
    /// Platform compatibility (optional - if None, works on all platforms)
    pub platforms: Option<Vec<String>>,
}

/// Registry containing all package managers that should be backed up
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageRegistry {
    /// Version of the registry format
    pub version: String,
    /// Map of entry IDs to package manager entries
    pub entries: HashMap<String, PackageManagerEntry>,
}

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
                platforms: None, // works on all platforms
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

        // pip global packages (system-wide)
        entries.insert(
            "pip".to_string(),
            PackageManagerEntry {
                name: "pip Packages".to_string(),
                command: "pip".to_string(),
                args: vec!["list".to_string(), "--format=freeze".to_string()],
                output_file: "pip.txt".to_string(),
                enabled: false, // disabled by default as it can be noisy
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
    /// Load registry from file, creating default if it doesn't exist
    pub fn load_or_create(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let registry: PackageRegistry = serde_json::from_str(&content)?;
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
    pub fn get_enabled_entries(&self) -> impl Iterator<Item = (&String, &PackageManagerEntry)> {
        self.entries.iter().filter(|(_, entry)| entry.enabled)
    }

    /// Get entries compatible with the current platform
    pub fn get_platform_compatible_entries(
        &self,
        current_platform: &str,
    ) -> impl Iterator<Item = (&String, &PackageManagerEntry)> {
        self.entries.iter().filter(move |(_, entry)| {
            entry.enabled
                && match &entry.platforms {
                    Some(platforms) => platforms.contains(&current_platform.to_string()),
                    None => true, // None means compatible with all platforms
                }
        })
    }

    /// Add a new entry
    pub fn add_entry(&mut self, id: String, entry: PackageManagerEntry) {
        self.entries.insert(id, entry);
    }

    /// Remove an entry
    pub fn remove_entry(&mut self, id: &str) -> Option<PackageManagerEntry> {
        self.entries.remove(id)
    }

    /// Enable/disable an entry
    pub fn set_entry_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        match self.entries.get_mut(id) {
            Some(entry) => {
                entry.enabled = enabled;
                Ok(())
            }
            None => Err(format!("Package manager entry '{}' not found", id)),
        }
    }

    /// Get entry by ID
    pub fn get_entry(&self, id: &str) -> Option<&PackageManagerEntry> {
        self.entries.get(id)
    }

    /// Update an existing entry
    pub fn update_entry(&mut self, id: &str, entry: PackageManagerEntry) -> Result<(), String> {
        match self.entries.get_mut(id) {
            Some(existing_entry) => {
                *existing_entry = entry;
                Ok(())
            }
            None => Err(format!("Package manager entry '{}' not found", id)),
        }
    }

    /// Get current platform string
    pub fn get_current_platform() -> String {
        #[cfg(target_os = "macos")]
        return "macos".to_string();

        #[cfg(target_os = "linux")]
        return "linux".to_string();

        #[cfg(target_os = "windows")]
        return "windows".to_string();

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        return "unknown".to_string();
    }
}
