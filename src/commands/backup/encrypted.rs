use crate::encryption::{encrypt_file, get_encrypted_path, prompt_password};
use crate::registry::encrypted::EncryptedRegistry;
use crate::utils::paths::get_encrypted_registry_path;
use age::secrecy::SecretString;
use std::fs;
use std::path::Path;

pub fn backup_encrypted_configs(encrypted_backup_path: &Path) {
    match prompt_password(true) {
        Ok(password) => {
            backup_encrypted_configs_with_password(encrypted_backup_path, &password);
        }
        Err(e) => {
            eprintln!("Skipping encrypted backup: {}", e);
        }
    }
}

fn backup_encrypted_configs_with_password(encrypted_backup_path: &Path, password: &SecretString) {
    let registry_path = get_encrypted_registry_path();
    let encrypted_registry = match EncryptedRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            eprintln!(
                "Failed to load encrypted registry, skipping encrypted backup: {}",
                e
            );
            return;
        }
    };

    let enabled_entries: Vec<_> = encrypted_registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("No encrypted configuration files found to backup");
        return;
    }

    println!(
        "Backing up {} encrypted configuration files...",
        enabled_entries.len()
    );

    for (_, entry) in enabled_entries {
        let target_path = &entry.target_path;

        if !target_path.exists() {
            eprintln!(
                "Source file for {} not found: {}",
                entry.name,
                target_path.display()
            );
            continue;
        }

        let encrypted_path = get_encrypted_path(&entry.source_path);
        let backup_destination = encrypted_backup_path.join(&encrypted_path);

        if let Some(parent) = backup_destination.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            eprintln!(
                "Failed to create backup directory for {}: {}",
                entry.name, e
            );
            continue;
        }

        match encrypt_file(target_path, &backup_destination, password) {
            Ok(()) => {
                println!(
                    "Backed up encrypted {} from {}",
                    entry.name,
                    target_path.display()
                );
            }
            Err(e) => {
                eprintln!("Failed to backup encrypted {}: {}", entry.name, e);
            }
        }
    }
}
