use crate::encryption::{encrypt_file, get_encrypted_path, prompt_password};
use crate::registry::encrypted::EncryptedRegistry;
use crate::utils::{
    display::{green, short_component, yellow},
    paths::get_encrypted_registry_path,
};
use age::secrecy::SecretString;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn backup_encrypted_configs(encrypted_backup_path: &Path) -> Result<(u32, u32)> {
    let password =
        prompt_password(true).context("Prompt for encryption password before encrypted backup")?;

    backup_encrypted_configs_with_password(encrypted_backup_path, &password)
}

fn backup_encrypted_configs_with_password(
    encrypted_backup_path: &Path,
    password: &SecretString,
) -> Result<(u32, u32)> {
    let registry_path = get_encrypted_registry_path();
    let encrypted_registry = EncryptedRegistry::load_or_create(&registry_path)
        .with_context(|| format!("Load encrypted registry: {}", registry_path.display()))?;

    let enabled_entries: Vec<_> = encrypted_registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("No encrypted configuration files found to backup");
        return Ok((0, 0));
    }

    println!("   Encrypted configs: {} entries", enabled_entries.len());

    let mut succeeded = 0;
    let mut skipped = 0;

    for (id, entry) in enabled_entries {
        if !entry.target_path.exists() {
            skipped += 1;
            println!(
                "{}",
                yellow(&format!(
                    "     skipped missing source {} ({})",
                    entry.source_path, id
                ))
            );
            continue;
        }

        let encrypted_path = get_encrypted_path(&entry.source_path);
        let backup_destination = encrypted_backup_path.join(&encrypted_path);
        let target_path = &entry.target_path;
        let target_label = short_component(target_path);

        let entry_result: Result<()> = (|| {
            if let Some(parent) = backup_destination.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Prepare encrypted backup path {} ({})",
                        parent.display(),
                        id
                    )
                })?;
            }

            encrypt_file(target_path, &backup_destination, password)
                .with_context(|| format!("Encrypt {} -> {}", target_label, entry.source_path))
        })();

        match entry_result {
            Ok(()) => {
                succeeded += 1;
                println!("     {} {}", green("✔"), entry.source_path);
            }
            Err(e) => {
                skipped += 1;
                println!(
                    "{}",
                    yellow(&format!(
                        "     skipped {} ({}): {}",
                        entry.source_path, id, e
                    ))
                );
            }
        }
    }

    Ok((succeeded, skipped))
}
