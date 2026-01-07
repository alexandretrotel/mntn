use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::{
    registry::{Registry, RegistryEntryLike},
    utils::paths::get_base_dirs,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedRegistryEntry {
    pub name: String,
    pub source_path: String,
    pub target_path: PathBuf,
    pub enabled: bool,
    pub description: Option<String>,
    pub encrypt_filename: bool,
}

use crate::impl_registry_entry_like;

impl_registry_entry_like!(EncryptedRegistryEntry);

pub type EncryptedConfigsRegistry = Registry<EncryptedRegistryEntry>;

impl Default for EncryptedConfigsRegistry {
    fn default() -> Self {
        let mut entries = HashMap::new();

        let base_dirs = get_base_dirs();
        let home_dir = base_dirs.home_dir();

        entries.insert(
            "ssh_config".to_string(),
            EncryptedRegistryEntry {
                name: "SSH Config".to_string(),
                source_path: "ssh/config".to_string(),
                target_path: home_dir.join(".ssh/config"),
                enabled: true,
                description: Some("SSH client configuration file".to_string()),
                encrypt_filename: false,
            },
        );

        entries.insert(
            "ssh_private_key".to_string(),
            EncryptedRegistryEntry {
                name: "SSH Private Key".to_string(),
                source_path: "ssh/id_ed25519".to_string(),
                target_path: home_dir.join(".ssh/id_ed25519"),
                enabled: true,
                description: Some("SSH Ed25519 private key".to_string()),
                encrypt_filename: true,
            },
        );

        Self {
            version: "1.0.0".to_string(),
            entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_entries() {
        let registry = EncryptedConfigsRegistry::default();
        assert!(!registry.entries.is_empty());
    }

    #[test]
    fn test_default_registry_has_ssh_config() {
        let registry = EncryptedConfigsRegistry::default();
        let entry = registry.get_entry("ssh_config");
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.name, "SSH Config");
        assert!(entry.enabled);
        assert!(!entry.encrypt_filename);
    }

    #[test]
    fn test_default_registry_has_ssh_private_key() {
        let registry = EncryptedConfigsRegistry::default();
        let entry = registry.get_entry("ssh_private_key");
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.name, "SSH Private Key");
        assert!(entry.enabled);
        assert!(entry.encrypt_filename);
    }

    #[test]
    fn test_registry_entry_like_implementation() {
        let mut entry = EncryptedRegistryEntry {
            name: "Test".to_string(),
            source_path: "test".to_string(),
            target_path: PathBuf::from("/test"),
            enabled: false,
            description: None,
            encrypt_filename: false,
        };

        assert!(!entry.is_enabled());
        entry.set_enabled(true);
        assert!(entry.is_enabled());
    }

    #[test]
    fn test_registry_get_enabled_entries() {
        let mut registry = EncryptedConfigsRegistry::default();
        registry.set_entry_enabled("ssh_config", false).unwrap();

        let enabled: Vec<_> = registry.get_enabled_entries().collect();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].0, "ssh_private_key");
    }
}
