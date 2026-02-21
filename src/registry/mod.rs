pub mod config;
pub mod encrypted;
pub mod package;

use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, collections::HashMap, path::PathBuf};

pub(crate) trait RegistryEntryLike {
    fn is_enabled(&self) -> bool;
}

#[macro_export]
macro_rules! impl_registry_entry_like {
    ($t:ty) => {
        impl RegistryEntryLike for $t {
            fn is_enabled(&self) -> bool {
                self.enabled
            }
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Registry<T> {
    pub version: String,
    pub entries: HashMap<String, T>,
}

impl<T> Registry<T>
where
    T: RegistryEntryLike + Clone + Serialize + for<'a> Deserialize<'a>,
{
    pub(crate) fn load_or_create(path: &PathBuf) -> Result<Self>
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

    pub(crate) fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let sorted_entries: BTreeMap<&String, &T> = self.entries.iter().collect();
        let sorted_registry = serde_json::json!({
            "version": self.version,
            "entries": sorted_entries
        });

        let content = serde_json::to_string_pretty(&sorted_registry)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub(crate) fn get_enabled_entries(&self) -> impl Iterator<Item = (&String, &T)> {
        self.entries.iter().filter(|(_, e)| e.is_enabled())
    }
}
