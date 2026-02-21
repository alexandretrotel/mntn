use crate::commands::validate::types::{ValidationError, Validator};
use crate::registry::config::ConfigRegistry;
use crate::registry::package::PackageRegistry;
use crate::utils::paths::{get_config_registry_path, get_package_registry_path};
use crate::utils::system::is_command_available;
use std::collections::HashMap;
use std::io::ErrorKind;

pub struct RegistryFilesValidator;

impl Validator for RegistryFilesValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        let config_registry_path = get_config_registry_path();
        match std::fs::read_to_string(&config_registry_path) {
            Ok(content) => match serde_json::from_str::<ConfigRegistry>(&content) {
                Ok(registry) => {
                    let mut source_paths: HashMap<String, Vec<String>> = HashMap::new();
                    for (id, entry) in registry.entries.iter() {
                        source_paths
                            .entry(entry.source_path.clone())
                            .or_default()
                            .push(id.clone());
                    }
                    for (path, ids) in source_paths {
                        if ids.len() > 1 {
                            errors.push(
                                ValidationError::warning(format!(
                                    "Duplicate source path '{}' used by: {}",
                                    path,
                                    ids.join(", ")
                                ))
                                .with_fix("Consider consolidating or renaming entries"),
                            );
                        }
                    }
                }
                Err(e) => {
                    errors.push(ValidationError::error(format!(
                        "Could not parse config registry: {}",
                        e
                    )));
                }
            },
            Err(e) if e.kind() == ErrorKind::NotFound => {
                errors.push(ValidationError::info(
                    "Config registry file not found".to_string(),
                ));
            }
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not read config registry: {}",
                    e
                )));
            }
        }

        let package_registry_path = get_package_registry_path();
        match std::fs::read_to_string(&package_registry_path) {
            Ok(content) => match serde_json::from_str::<PackageRegistry>(&content) {
                Ok(registry) => {
                    let current_platform = PackageRegistry::get_current_platform();
                    for (id, entry) in registry.get_platform_compatible_entries(&current_platform) {
                        if !is_command_available(&entry.command) {
                            errors.push(
                                ValidationError::info(format!(
                                    "Package manager '{}' ({}) not found in PATH",
                                    entry.name, id
                                ))
                                .with_fix(format!(
                                    "Install {} or disable this entry in your profile config",
                                    entry.command
                                )),
                            );
                        }
                    }
                }
                Err(e) => {
                    errors.push(ValidationError::error(format!(
                        "Could not parse package registry: {}",
                        e
                    )));
                }
            },
            Err(e) if e.kind() == ErrorKind::NotFound => {
                errors.push(ValidationError::info(
                    "Package registry file not found".to_string(),
                ));
            }
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not read package registry: {}",
                    e
                )));
            }
        }

        errors
    }

    fn name(&self) -> &str {
        "Registry Files"
    }
}
