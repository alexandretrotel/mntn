use crate::commands::validate::types::{ValidationError, Validator};
use crate::commands::validate::utils::validate_json_file;
use crate::profiles::ActiveProfile;
use crate::registry::config::ConfigRegistry;
use crate::utils::paths::get_config_registry_path;

pub struct JsonFilesValidator {
    profile: ActiveProfile,
}

impl JsonFilesValidator {
    pub fn new(profile: ActiveProfile) -> Self {
        Self { profile }
    }
}

impl Validator for JsonFilesValidator {
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

        for (_id, entry) in config_registry.get_enabled_entries() {
            if entry.source_path.ends_with(".json")
                && let Some(resolved) = self.profile.resolve_source(&entry.source_path)
            {
                errors.extend(validate_json_file(&resolved.path, &entry.name));
            }
        }

        errors
    }

    fn name(&self) -> &str {
        "JSON Configuration Files"
    }
}
