use crate::encryption::{decrypt_file, get_encrypted_path};
use crate::profiles::ActiveProfile;
use crate::registry::encrypted::EncryptedRegistry;
use crate::utils::paths::get_encrypted_registry_path;
use age::secrecy::SecretString;
use std::fs;

pub fn restore_encrypted_configs(profile: &ActiveProfile, password: &SecretString) -> (u32, u32) {
    let encrypted_registry_path = get_encrypted_registry_path();
    let encrypted_registry = match EncryptedRegistry::load_or_create(&encrypted_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            eprintln!(
                "Failed to load encrypted registry, skipping encrypted restore: {}",
                e
            );
            return (0, 0);
        }
    };

    let enabled_entries: Vec<_> = encrypted_registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("No encrypted configuration files found to restore");
        return (0, 0);
    }

    println!(
        "Restoring {} encrypted configuration files...",
        enabled_entries.len()
    );

    let mut restored_count = 0;
    let mut skipped_count = 0;

    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;
        let encrypted_path = get_encrypted_path(&entry.source_path);

        match profile.resolve_encrypted_source(&encrypted_path) {
            Some(resolved) => {
                println!("Restoring: {} ({}) [{}]", entry.name, id, resolved.layer);

                if target_path.is_symlink() {
                    if let Err(e) = fs::remove_file(target_path) {
                        eprintln!("Failed to remove legacy symlink for {}: {}", entry.name, e);
                        skipped_count += 1;
                        continue;
                    }
                    println!("Removed legacy symlink at {}", target_path.display());
                }

                if let Some(parent) = target_path.parent()
                    && let Err(e) = fs::create_dir_all(parent)
                {
                    eprintln!("Failed to create directory for {}: {}", entry.name, e);
                    skipped_count += 1;
                    continue;
                }

                match decrypt_file(&resolved.path, target_path, password) {
                    Ok(()) => {
                        println!("Restored encrypted {}", entry.name);
                        restored_count += 1;
                    }
                    Err(e) => {
                        eprintln!("Failed to restore encrypted {}: {}", entry.name, e);
                        skipped_count += 1;
                    }
                }
            }
            None => {
                println!("No encrypted backup found for {} in any layer", entry.name);
                skipped_count += 1;
            }
        }
    }

    (restored_count, skipped_count)
}
