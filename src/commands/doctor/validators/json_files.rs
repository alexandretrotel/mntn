use crate::commands::doctor::types::{ValidationError, Validator};
use crate::commands::doctor::utils::{enabled_json_files, validate_json_file};
use crate::profiles::ActiveProfile;

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
        let files = match enabled_json_files(&self.profile) {
            Ok(f) => f,
            Err(e) => {
                return vec![ValidationError::error(format!(
                    "Could not load config registry: {}",
                    e
                ))];
            }
        };

        let mut errors = Vec::new();
        for (path, name) in files {
            errors.extend(validate_json_file(&path, &name));
        }

        errors
    }

    fn name(&self) -> &str {
        "JSON Configuration Files"
    }
}
