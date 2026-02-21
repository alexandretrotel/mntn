use crate::commands::validate::types::{ValidationError, Validator};
use crate::profiles::ActiveProfile;
use crate::registry::config::ConfigRegistry;
use crate::utils::paths::get_config_registry_path;

pub struct LayerResolutionValidator {
    profile: ActiveProfile,
}

impl LayerResolutionValidator {
    pub fn new(profile: ActiveProfile) -> Self {
        Self { profile }
    }
}

impl Validator for LayerResolutionValidator {
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
            let all_sources = self.profile.get_all_resolved_sources(&entry.source_path);

            if all_sources.is_empty() {
                continue;
            }

            let primary = &all_sources[0];

            if all_sources.len() > 1 {
                let layers: Vec<String> = all_sources.iter().map(|s| s.layer.to_string()).collect();
                errors.push(
                    ValidationError::info(format!(
                        "{} ({}): Found in multiple layers: {} (using {})",
                        entry.name,
                        id,
                        layers.join(", "),
                        primary.layer
                    ))
                    .with_fix("This is expected for overrides. Higher-priority layer wins."),
                );
            }
        }

        errors
    }

    fn name(&self) -> &str {
        "Layer Resolution"
    }
}
