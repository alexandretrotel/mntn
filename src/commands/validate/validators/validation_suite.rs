use crate::commands::validate::types::{ValidationReport, Validator};
use crate::profiles::ActiveProfile;

use super::backup_consistency::BackupConsistencyValidator;
use super::json_files::JsonFilesValidator;
use super::layer_resolution::LayerResolutionValidator;
use super::registry_files::RegistryFilesValidator;

pub struct ValidationSuite {
    validators: Vec<Box<dyn Validator>>,
}

impl ValidationSuite {
    pub fn new(profile: ActiveProfile, skip_encrypted: bool) -> Self {
        let validators: Vec<Box<dyn Validator>> = vec![
            Box::new(RegistryFilesValidator),
            Box::new(LayerResolutionValidator::new(profile.clone())),
            Box::new(JsonFilesValidator::new(profile.clone())),
            Box::new(BackupConsistencyValidator::new(
                profile.clone(),
                skip_encrypted,
            )),
        ];
        Self { validators }
    }

    pub fn run_all(&self) -> ValidationReport {
        let mut report = ValidationReport::new();
        for validator in &self.validators {
            let errors = validator.validate();
            report.add_result(validator.name(), errors);
        }
        report
    }
}
