use crate::commands::validate::types::{ValidationError, Validator};
use crate::commands::validate::utils::create_temp_file_path;
use crate::encryption::{decrypt_file, get_encrypted_path, prompt_password};
use crate::profiles::ActiveProfile;
use crate::registry::config::ConfigRegistry;
use crate::registry::encrypted::EncryptedRegistry;
use crate::utils::paths::{get_config_registry_path, get_encrypted_registry_path};
use std::fs;

pub struct BackupConsistencyValidator {
    profile: ActiveProfile,
    skip_encrypted: bool,
}

impl BackupConsistencyValidator {
    pub fn new(profile: ActiveProfile, skip_encrypted: bool) -> Self {
        Self {
            profile,
            skip_encrypted,
        }
    }
}

impl Validator for BackupConsistencyValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        let config_registry_path = get_config_registry_path();
        let config_registry = match ConfigRegistry::load_or_create(&config_registry_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not load config registry: {}",
                    e
                )));
                return errors;
            }
        };

        for (id, entry) in config_registry.get_enabled_entries() {
            if !entry.target_path.exists() {
                continue;
            }

            if entry.target_path.is_dir() {
                continue;
            }

            if let Some(resolved) = self.profile.resolve_source(&entry.source_path) {
                if !resolved.path.exists() {
                    continue;
                }

                if resolved.path.is_dir() {
                    continue;
                }

                let backup_content = match fs::read(&resolved.path) {
                    Ok(content) => content,
                    Err(e) => {
                        errors.push(ValidationError::warning(format!(
                            "Could not read backup file for {} ({}): {}",
                            entry.name, id, e
                        )));
                        continue;
                    }
                };

                let current_content = match fs::read(&entry.target_path) {
                    Ok(content) => content,
                    Err(e) => {
                        errors.push(ValidationError::warning(format!(
                            "Could not read current file for {} ({}): {}",
                            entry.name, id, e
                        )));
                        continue;
                    }
                };

                if backup_content != current_content {
                    errors.push(
                        ValidationError::warning(format!(
                            "{} ({}): File differs from backup",
                            entry.name, id
                        ))
                        .with_fix("Run 'mntn backup' to update backup or 'mntn restore' to restore from backup"),
                    );
                }
            }
        }

        if self.skip_encrypted {
            return errors;
        }

        let encrypted_registry_path = get_encrypted_registry_path();
        let encrypted_registry = match EncryptedRegistry::load_or_create(&encrypted_registry_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not load encrypted config registry: {}",
                    e
                )));
                return errors;
            }
        };

        let mut entries_to_validate = Vec::new();
        for (id, entry) in encrypted_registry.get_enabled_entries() {
            if !entry.target_path.exists() {
                continue;
            }

            if entry.target_path.is_dir() {
                continue;
            }

            let encrypted_path = get_encrypted_path(&entry.source_path);

            if let Some(resolved) = self.profile.resolve_encrypted_source(&encrypted_path) {
                if !resolved.path.exists() {
                    continue;
                }

                if resolved.path.is_dir() {
                    continue;
                }

                entries_to_validate.push((id.clone(), entry.clone(), resolved));
            }
        }

        if entries_to_validate.is_empty() {
            return errors;
        }

        let password = match prompt_password(false) {
            Ok(pwd) => pwd,
            Err(e) => {
                errors.push(ValidationError::warning(format!(
                    "Skipping encrypted file validation: {}",
                    e
                )));
                return errors;
            }
        };

        for (id, entry, resolved) in entries_to_validate {
            let temp_path = match create_temp_file_path() {
                Ok(path) => path,
                Err(e) => {
                    errors.push(ValidationError::warning(format!(
                        "Could not create temporary file for {} ({}): {}",
                        entry.name, id, e
                    )));
                    continue;
                }
            };

            match decrypt_file(&resolved.path, &temp_path, &password) {
                Ok(()) => {
                    let backup_content = match fs::read(&temp_path) {
                        Ok(content) => content,
                        Err(e) => {
                            errors.push(ValidationError::warning(format!(
                                "Could not read decrypted backup file for {} ({}): {}",
                                entry.name, id, e
                            )));
                            let _ = fs::remove_file(&temp_path);
                            continue;
                        }
                    };

                    let current_content = match fs::read(&entry.target_path) {
                        Ok(content) => content,
                        Err(e) => {
                            errors.push(ValidationError::warning(format!(
                                "Could not read current file for {} ({}): {}",
                                entry.name, id, e
                            )));
                            let _ = fs::remove_file(&temp_path);
                            continue;
                        }
                    };

                    if backup_content != current_content {
                        errors.push(
                            ValidationError::warning(format!(
                                "{} ({}): Encrypted file differs from backup",
                                entry.name, id
                            ))
                            .with_fix("Run 'mntn backup' to update backup or 'mntn restore' to restore from backup"),
                        );
                    }
                    let _ = fs::remove_file(&temp_path);
                }
                Err(e) => {
                    let error_msg = e.to_string().to_lowercase();
                    if error_msg.contains("password")
                        || error_msg.contains("decrypt")
                        || error_msg.contains("identity")
                    {
                        errors.push(ValidationError::warning(
                            "Skipping encrypted file validation: Incorrect password".to_string(),
                        ));
                        return errors;
                    } else {
                        errors.push(ValidationError::warning(format!(
                            "Could not decrypt backup file for {} ({}): {}",
                            entry.name, id, e
                        )));
                    }
                    let _ = fs::remove_file(&temp_path);
                }
            }
        }

        errors
    }

    fn name(&self) -> &str {
        "Backup Consistency Check"
    }
}
