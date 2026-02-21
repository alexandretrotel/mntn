use super::utils::{backup_directory, backup_file};
use crate::registry::config::ConfigRegistry;
use crate::utils::paths::get_config_registry_path;
use std::fs;
use std::path::Path;

pub fn backup_configs(configs_path: &Path) {
    let config_registry_path = get_config_registry_path();
    let config_registry = match ConfigRegistry::load_or_create(&config_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            eprintln!(
                "Failed to load registry, skipping config file backup: {}",
                e
            );
            return;
        }
    };

    let enabled_entries: Vec<_> = config_registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("No configuration files found to backup");
        return;
    }

    println!(
        "Backing up {} configuration files...",
        enabled_entries.len()
    );

    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;
        let backup_destination = configs_path.join(&entry.source_path);

        if let Some(parent) = backup_destination.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            eprintln!(
                "Failed to create backup directory for {}: {}",
                entry.name, e
            );
            continue;
        }

        let result = if target_path.is_dir() {
            backup_directory(target_path, &backup_destination)
        } else {
            backup_file(target_path, &backup_destination)
        };

        match result {
            Ok(()) => {
                println!("Backed up {} ({})", entry.name, id);
                println!("Backed up {} from {}", entry.name, target_path.display());
            }
            Err(e) => {
                eprintln!("Failed to backup {}: {}", entry.name, e);
            }
        }
    }
}
