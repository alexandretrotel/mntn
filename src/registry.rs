use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

/// Common interface for registry entries
pub trait RegistryEntryLike {
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
}

/// Macro to implement RegistryEntryLike for types with an `enabled` field
#[macro_export]
macro_rules! impl_registry_entry_like {
    ($t:ty) => {
        impl RegistryEntryLike for $t {
            fn is_enabled(&self) -> bool {
                self.enabled
            }
            fn set_enabled(&mut self, enabled: bool) {
                self.enabled = enabled;
            }
        }
    };
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
    /// Load registry from file, creating default if it doesn't exist
    pub fn load_or_create(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Default,
    {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let registry: Registry<T> = serde_json::from_str(&content)?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Test implementation of RegistryEntryLike for testing
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestEntry {
        name: String,
        enabled: bool,
        value: i32,
    }

    impl RegistryEntryLike for TestEntry {
        fn is_enabled(&self) -> bool {
            self.enabled
        }

        fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }
    }

    impl Default for Registry<TestEntry> {
        fn default() -> Self {
            Registry {
                version: "1.0.0".to_string(),
                entries: HashMap::new(),
            }
        }
    }

    fn create_test_entry(name: &str, enabled: bool, value: i32) -> TestEntry {
        TestEntry {
            name: name.to_string(),
            enabled,
            value,
        }
    }

    #[test]
    fn test_registry_entry_like_is_enabled() {
        let entry = create_test_entry("test", true, 42);
        assert!(entry.is_enabled());

        let disabled_entry = create_test_entry("test", false, 42);
        assert!(!disabled_entry.is_enabled());
    }

    #[test]
    fn test_registry_entry_like_set_enabled() {
        let mut entry = create_test_entry("test", false, 42);
        assert!(!entry.is_enabled());

        entry.set_enabled(true);
        assert!(entry.is_enabled());

        entry.set_enabled(false);
        assert!(!entry.is_enabled());
    }

    #[test]
    fn test_registry_default() {
        let registry: Registry<TestEntry> = Registry::default();
        assert_eq!(registry.version, "1.0.0");
        assert!(registry.entries.is_empty());
    }

    #[test]
    fn test_registry_with_entries() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("entry1".to_string(), create_test_entry("Entry 1", true, 1));
        registry.add_entry("entry2".to_string(), create_test_entry("Entry 2", false, 2));

        assert_eq!(registry.entries.len(), 2);
    }

    #[test]
    fn test_load_or_create_creates_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir.path().join("registry.json");

        assert!(!registry_path.exists());

        let registry: Registry<TestEntry> = Registry::load_or_create(&registry_path).unwrap();

        assert!(registry_path.exists());
        assert_eq!(registry.version, "1.0.0");
        assert!(registry.entries.is_empty());
    }

    #[test]
    fn test_load_or_create_loads_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir.path().join("registry.json");

        // Create and save a registry
        let mut original: Registry<TestEntry> = Registry {
            version: "2.0.0".to_string(),
            ..Default::default()
        };
        original.add_entry("test".to_string(), create_test_entry("Test", true, 100));
        original.save(&registry_path).unwrap();

        // Load it back
        let loaded: Registry<TestEntry> = Registry::load_or_create(&registry_path).unwrap();

        assert_eq!(loaded.version, "2.0.0");
        assert_eq!(loaded.entries.len(), 1);
        assert!(loaded.get_entry("test").is_some());
    }

    #[test]
    fn test_load_or_create_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("registry.json");

        let _registry: Registry<TestEntry> = Registry::load_or_create(&registry_path).unwrap();

        assert!(registry_path.exists());
    }

    #[test]
    fn test_load_or_create_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir.path().join("registry.json");

        fs::write(&registry_path, "{ invalid json }").unwrap();

        let result: Result<Registry<TestEntry>, _> = Registry::load_or_create(&registry_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir.path().join("registry.json");

        let registry: Registry<TestEntry> = Registry::default();
        registry.save(&registry_path).unwrap();

        assert!(registry_path.exists());
    }

    #[test]
    fn test_save_writes_valid_json() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir.path().join("registry.json");

        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry(
            "test".to_string(),
            create_test_entry("Test Entry", true, 42),
        );
        registry.save(&registry_path).unwrap();

        let content = fs::read_to_string(&registry_path).unwrap();
        assert!(content.contains("\"version\""));
        assert!(content.contains("\"entries\""));
        assert!(content.contains("Test Entry"));
    }

    #[test]
    fn test_save_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir
            .path()
            .join("a")
            .join("b")
            .join("c")
            .join("registry.json");

        let registry: Registry<TestEntry> = Registry::default();
        registry.save(&registry_path).unwrap();

        assert!(registry_path.exists());
    }

    #[test]
    fn test_save_overwrites_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir.path().join("registry.json");

        // Save first version
        let registry1: Registry<TestEntry> = Registry {
            version: "1.0.0".to_string(),
            ..Default::default()
        };
        registry1.save(&registry_path).unwrap();

        // Save second version
        let registry2: Registry<TestEntry> = Registry {
            version: "2.0.0".to_string(),
            ..Default::default()
        };
        registry2.save(&registry_path).unwrap();

        // Load and verify
        let loaded: Registry<TestEntry> = Registry::load_or_create(&registry_path).unwrap();
        assert_eq!(loaded.version, "2.0.0");
    }

    #[test]
    fn test_get_enabled_entries_empty() {
        let registry: Registry<TestEntry> = Registry::default();
        let enabled: Vec<_> = registry.get_enabled_entries().collect();
        assert!(enabled.is_empty());
    }

    #[test]
    fn test_get_enabled_entries_all_disabled() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("a".to_string(), create_test_entry("A", false, 1));
        registry.add_entry("b".to_string(), create_test_entry("B", false, 2));

        let enabled: Vec<_> = registry.get_enabled_entries().collect();
        assert!(enabled.is_empty());
    }

    #[test]
    fn test_get_enabled_entries_all_enabled() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("a".to_string(), create_test_entry("A", true, 1));
        registry.add_entry("b".to_string(), create_test_entry("B", true, 2));

        let enabled: Vec<_> = registry.get_enabled_entries().collect();
        assert_eq!(enabled.len(), 2);
    }

    #[test]
    fn test_get_enabled_entries_mixed() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("enabled1".to_string(), create_test_entry("E1", true, 1));
        registry.add_entry("disabled".to_string(), create_test_entry("D", false, 2));
        registry.add_entry("enabled2".to_string(), create_test_entry("E2", true, 3));

        let enabled: Vec<_> = registry.get_enabled_entries().collect();
        assert_eq!(enabled.len(), 2);

        // Check that all returned entries are enabled
        for (_, entry) in &enabled {
            assert!(entry.is_enabled());
        }
    }

    #[test]
    fn test_add_entry_new() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("new".to_string(), create_test_entry("New", true, 1));

        assert_eq!(registry.entries.len(), 1);
        assert!(registry.get_entry("new").is_some());
    }

    #[test]
    fn test_add_entry_overwrites_existing() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("key".to_string(), create_test_entry("Original", true, 1));
        registry.add_entry(
            "key".to_string(),
            create_test_entry("Replacement", false, 2),
        );

        assert_eq!(registry.entries.len(), 1);
        let entry = registry.get_entry("key").unwrap();
        assert_eq!(entry.name, "Replacement");
        assert_eq!(entry.value, 2);
    }

    #[test]
    fn test_add_entry_multiple() {
        let mut registry: Registry<TestEntry> = Registry::default();
        for i in 0..10 {
            registry.add_entry(
                format!("entry_{}", i),
                create_test_entry(&format!("Entry {}", i), true, i),
            );
        }

        assert_eq!(registry.entries.len(), 10);
    }

    #[test]
    fn test_remove_entry_exists() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry(
            "to_remove".to_string(),
            create_test_entry("Remove Me", true, 1),
        );

        let removed = registry.remove_entry("to_remove");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Remove Me");
        assert!(registry.get_entry("to_remove").is_none());
    }

    #[test]
    fn test_remove_entry_not_exists() {
        let mut registry: Registry<TestEntry> = Registry::default();
        let removed = registry.remove_entry("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_remove_entry_from_multiple() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("a".to_string(), create_test_entry("A", true, 1));
        registry.add_entry("b".to_string(), create_test_entry("B", true, 2));
        registry.add_entry("c".to_string(), create_test_entry("C", true, 3));

        registry.remove_entry("b");

        assert_eq!(registry.entries.len(), 2);
        assert!(registry.get_entry("a").is_some());
        assert!(registry.get_entry("b").is_none());
        assert!(registry.get_entry("c").is_some());
    }

    #[test]
    fn test_set_entry_enabled_to_true() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("test".to_string(), create_test_entry("Test", false, 1));

        let result = registry.set_entry_enabled("test", true);
        assert!(result.is_ok());
        assert!(registry.get_entry("test").unwrap().is_enabled());
    }

    #[test]
    fn test_set_entry_enabled_to_false() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("test".to_string(), create_test_entry("Test", true, 1));

        let result = registry.set_entry_enabled("test", false);
        assert!(result.is_ok());
        assert!(!registry.get_entry("test").unwrap().is_enabled());
    }

    #[test]
    fn test_set_entry_enabled_not_found() {
        let mut registry: Registry<TestEntry> = Registry::default();
        let result = registry.set_entry_enabled("nonexistent", true);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_set_entry_enabled_toggle() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("toggle".to_string(), create_test_entry("Toggle", true, 1));

        registry.set_entry_enabled("toggle", false).unwrap();
        assert!(!registry.get_entry("toggle").unwrap().is_enabled());

        registry.set_entry_enabled("toggle", true).unwrap();
        assert!(registry.get_entry("toggle").unwrap().is_enabled());
    }

    #[test]
    fn test_get_entry_exists() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry("test".to_string(), create_test_entry("Test", true, 42));

        let entry = registry.get_entry("test");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, 42);
    }

    #[test]
    fn test_get_entry_not_exists() {
        let registry: Registry<TestEntry> = Registry::default();
        assert!(registry.get_entry("nonexistent").is_none());
    }

    #[test]
    fn test_get_entry_returns_reference() {
        let mut registry: Registry<TestEntry> = Registry::default();
        registry.add_entry(
            "ref_test".to_string(),
            create_test_entry("Ref Test", true, 100),
        );

        let entry1 = registry.get_entry("ref_test");
        let entry2 = registry.get_entry("ref_test");

        assert_eq!(entry1.unwrap().value, entry2.unwrap().value);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir.path().join("registry.json");

        let mut original: Registry<TestEntry> = Registry {
            version: "test-version".to_string(),
            ..Default::default()
        };
        original.add_entry("entry1".to_string(), create_test_entry("Entry 1", true, 10));
        original.add_entry(
            "entry2".to_string(),
            create_test_entry("Entry 2", false, 20),
        );
        original.add_entry("entry3".to_string(), create_test_entry("Entry 3", true, 30));

        original.save(&registry_path).unwrap();

        let loaded: Registry<TestEntry> = Registry::load_or_create(&registry_path).unwrap();

        assert_eq!(loaded.version, original.version);
        assert_eq!(loaded.entries.len(), original.entries.len());

        for (id, original_entry) in &original.entries {
            let loaded_entry = loaded.get_entry(id).unwrap();
            assert_eq!(loaded_entry.name, original_entry.name);
            assert_eq!(loaded_entry.enabled, original_entry.enabled);
            assert_eq!(loaded_entry.value, original_entry.value);
        }
    }

    #[test]
    fn test_registry_clone() {
        let mut original: Registry<TestEntry> = Registry::default();
        original.add_entry("test".to_string(), create_test_entry("Test", true, 1));

        let cloned = original.clone();

        assert_eq!(cloned.version, original.version);
        assert_eq!(cloned.entries.len(), original.entries.len());
    }
}
