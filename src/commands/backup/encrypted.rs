use crate::encryption::{encrypt_file, get_encrypted_path, prompt_password};
use crate::registry::encrypted::EncryptedRegistry;
use crate::utils::paths::get_encrypted_registry_path;
use age::secrecy::SecretString;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn backup_encrypted_configs(encrypted_backup_path: &Path) -> Result<()> {
    let password =
        prompt_password(true).context("Prompt for encryption password before encrypted backup")?;

    backup_encrypted_configs_with_password(encrypted_backup_path, &password)
}

fn backup_encrypted_configs_with_password(
    encrypted_backup_path: &Path,
    password: &SecretString,
) -> Result<()> {
    let registry_path = get_encrypted_registry_path();
    let encrypted_registry = EncryptedRegistry::load_or_create(&registry_path)
        .with_context(|| format!("Load encrypted registry: {}", registry_path.display()))?;

    let enabled_entries: Vec<_> = encrypted_registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("No encrypted configuration files found to backup");
        return Ok(());
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

        if let Some(parent) = backup_destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create backup directory for {}: {}",
                    entry.name,
                    parent.display()
                )
            })?;
        }

        encrypt_file(target_path, &backup_destination, password).with_context(|| {
            format!(
                "Failed to encrypt {} from {}",
                entry.name,
                target_path.display()
            )
        })?;

        println!(
            "Backed up encrypted {} from {}",
            entry.name,
            target_path.display()
        );
    }

    Ok(())
}
