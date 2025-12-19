use crate::logger::{log, log_info, log_success, log_warning};
use crate::profile::{ActiveProfile, ProfileConfig};
use crate::registries::configs_registry::ConfigsRegistry;
use crate::registries::package_registry::PackageRegistry;
use crate::tasks::core::{PlannedOperation, Task, TaskExecutor};
use crate::utils::paths::{get_backup_root, get_package_registry_path, get_registry_path};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Severity level for validation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
            Severity::Warning => write!(f, "WARNING"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}

/// A validation error with severity, message, and optional fix suggestion
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub severity: Severity,
    pub message: String,
    pub fix_suggestion: Option<String>,
}

impl ValidationError {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            fix_suggestion: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            fix_suggestion: None,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            message: message.into(),
            fix_suggestion: None,
        }
    }

    pub fn with_fix(mut self, suggestion: impl Into<String>) -> Self {
        self.fix_suggestion = Some(suggestion.into());
        self
    }
}

/// Helper function to validate JSON syntax in a file
fn validate_json_file(path: &Path, description: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if !path.exists() {
        return errors;
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(
                ValidationError::warning(format!("Could not read {}: {}", description, e))
                    .with_fix(format!("Check file permissions for {}", path.display())),
            );
            return errors;
        }
    };
    if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
        errors.push(
            ValidationError::error(format!("Invalid JSON in {}: {}", description, e))
                .with_fix(format!("Check syntax in {}", path.display())),
        );
    }
    errors
}

/// Trait for implementing validators
pub trait Validator {
    fn validate(&self) -> Vec<ValidationError>;
    fn name(&self) -> &str;
}

/// Report containing all validation results
#[derive(Default)]
pub struct ValidationReport {
    results: Vec<(String, Vec<ValidationError>)>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_result(&mut self, validator_name: &str, errors: Vec<ValidationError>) {
        self.results.push((validator_name.to_string(), errors));
    }

    fn count_by_severity(&self, severity: Severity) -> usize {
        self.results
            .iter()
            .flat_map(|(_, errors)| errors.iter())
            .filter(|e| e.severity == severity)
            .count()
    }

    pub fn error_count(&self) -> usize {
        self.count_by_severity(Severity::Error)
    }

    pub fn warning_count(&self) -> usize {
        self.count_by_severity(Severity::Warning)
    }

    pub fn print(&self) {
        for (name, errors) in &self.results {
            if errors.is_empty() {
                println!(" {} OK", name);
            } else {
                println!(" {}", name);
                for error in errors {
                    let icon = match error.severity {
                        Severity::Error => " x",
                        Severity::Warning => " !",
                        Severity::Info => " i",
                    };
                    println!("{} {}", icon, error.message);
                    if let Some(fix) = &error.fix_suggestion {
                        println!(" Fix: {}", fix);
                    }
                }
            }
        }
    }
}

/// Validates JSON configuration files
pub struct JsonConfigValidator {
    profile: ActiveProfile,
}

impl JsonConfigValidator {
    pub fn new(profile: ActiveProfile) -> Self {
        Self { profile }
    }
}

impl Validator for JsonConfigValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        let registry_path = get_registry_path();
        let registry = match ConfigsRegistry::load_or_create(&registry_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not load configs registry: {}",
                    e
                )));
                return errors;
            }
        };

        for (_id, entry) in registry.get_enabled_entries() {
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

/// Checks for legacy symlinks that should be converted to real files.
/// This validator warns users who previously used symlink-based management
/// that they should run backup or restore to convert to real files.
pub struct LegacySymlinkValidator {}

impl LegacySymlinkValidator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Validator for LegacySymlinkValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let registry_path = get_registry_path();
        let registry = match ConfigsRegistry::load_or_create(&registry_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not load configs registry: {}",
                    e
                )));
                return errors;
            }
        };

        let backup_root = get_backup_root();
        let mut symlink_count = 0;

        for (id, entry) in registry.get_enabled_entries() {
            let target_path = &entry.target_path;

            // Check if target is a symlink pointing to our backup
            if target_path.is_symlink()
                && let Ok(link_target) = fs::read_link(target_path)
            {
                let canonical_target = link_target
                    .canonicalize()
                    .unwrap_or_else(|_| link_target.clone());

                // Check if the symlink target is within our backup directory
                if canonical_target.starts_with(&backup_root) {
                    errors.push(
                        ValidationError::warning(format!(
                            "{} ({}): Legacy symlink detected",
                            entry.name, id
                        ))
                        .with_fix("Run 'mntn backup' or 'mntn restore' to convert to a real file"),
                    );
                    symlink_count += 1;
                }
            }
        }

        if symlink_count > 0 {
            errors.push(
                ValidationError::info(format!(
                    "Found {} legacy symlink(s) from previous mntn version",
                    symlink_count
                ))
                .with_fix("Run 'mntn migrate' to convert all symlinks to real files"),
            );
        }

        errors
    }

    fn name(&self) -> &str {
        "Legacy Symlink Check"
    }
}

/// Validates and reports which layer each config is resolved from
pub struct LayerValidator {
    profile: ActiveProfile,
}

impl LayerValidator {
    pub fn new(profile: ActiveProfile) -> Self {
        Self { profile }
    }
}

impl Validator for LayerValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let registry_path = get_registry_path();
        let registry = match ConfigsRegistry::load_or_create(&registry_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not load configs registry: {}",
                    e
                )));
                return errors;
            }
        };

        let backup_root = get_backup_root();
        let mut has_legacy = false;

        for (id, entry) in registry.get_enabled_entries() {
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

            if primary.layer == crate::profile::SourceLayer::Legacy {
                has_legacy = true;
            }
        }

        if has_legacy {
            let legacy_path = backup_root.display();
            errors.push(
                ValidationError::warning(format!(
                    "Some configs are still in legacy location ({})",
                    legacy_path
                ))
                .with_fix("Run 'mntn migrate' to migrate to the layered structure"),
            );
        }

        errors
    }

    fn name(&self) -> &str {
        "Layer Resolution"
    }
}

/// Validates registry files are valid and consistent
pub struct RegistryValidator;

impl Validator for RegistryValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        let registry_path = get_registry_path();
        match ConfigsRegistry::load_or_create(&registry_path) {
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
                    "Could not load configs registry: {}",
                    e
                )));
            }
        }

        let package_registry_path = get_package_registry_path();
        match PackageRegistry::load_or_create(&package_registry_path) {
            Ok(registry) => {
                let current_platform = PackageRegistry::get_current_platform();
                for (id, entry) in registry.get_platform_compatible_entries(&current_platform) {
                    if which::which(&entry.command).is_err() {
                        errors.push(
                            ValidationError::info(format!(
                                "Package manager '{}' ({}) not found in PATH",
                                entry.name, id
                            ))
                            .with_fix(format!(
                                "Install {} or disable this entry with 'mntn registry-packages toggle {} -e false'",
                                entry.command, id
                            )),
                        );
                    }
                }
            }
            Err(e) => {
                errors.push(ValidationError::error(format!(
                    "Could not load package registry: {}",
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

/// Main validator that runs all validators
pub struct ConfigValidator {
    validators: Vec<Box<dyn Validator>>,
}

impl ConfigValidator {
    pub fn new(profile: ActiveProfile) -> Self {
        let validators: Vec<Box<dyn Validator>> = vec![
            Box::new(RegistryValidator),
            Box::new(LayerValidator::new(profile.clone())),
            Box::new(JsonConfigValidator::new(profile.clone())),
            Box::new(LegacySymlinkValidator::new()),
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

/// Validation task
pub struct ValidateTask {
    profile: ActiveProfile,
}

impl ValidateTask {
    pub fn new(profile: ActiveProfile) -> Self {
        Self { profile }
    }
}

impl Task for ValidateTask {
    fn name(&self) -> &str {
        "Validate"
    }

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Validating configuration...");
        println!("   Profile: {}", self.profile);
        log("Starting validation");

        let validator = ConfigValidator::new(self.profile.clone());
        let report = validator.run_all();
        println!();
        report.print();
        println!();
        let error_count = report.error_count();
        let warning_count = report.warning_count();
        if error_count == 0 && warning_count == 0 {
            log_success("All checks passed");
        } else {
            log_warning(&format!(
                "Validation complete: {} error(s), {} warning(s)",
                error_count, warning_count
            ));
        }
        Ok(())
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        vec![
            PlannedOperation::new("Validate registry files"),
            PlannedOperation::new("Validate layer resolution"),
            PlannedOperation::new("Validate JSON configuration files"),
            PlannedOperation::new("Check for legacy symlinks"),
        ]
    }
}

pub fn run_with_args(args: crate::cli::ValidateArgs) {
    if let Ok(true) = ProfileConfig::save_default_if_missing() {
        log_info("Created default profile config at ~/.mntn/profile.json");
    }

    let profile = args.resolve_profile();
    TaskExecutor::run(&mut ValidateTask::new(profile), args.dry_run);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Error.to_string(), "ERROR");
        assert_eq!(Severity::Warning.to_string(), "WARNING");
        assert_eq!(Severity::Info.to_string(), "INFO");
    }

    #[test]
    fn test_severity_equality() {
        assert_eq!(Severity::Error, Severity::Error);
        assert_ne!(Severity::Error, Severity::Warning);
        assert_ne!(Severity::Warning, Severity::Info);
    }

    #[test]
    fn test_severity_clone() {
        let severity = Severity::Warning;
        let cloned = severity;
        assert_eq!(severity, cloned);
    }

    #[test]
    fn test_validation_error_error() {
        let err = ValidationError::error("Test error message");
        assert_eq!(err.severity, Severity::Error);
        assert_eq!(err.message, "Test error message");
        assert!(err.fix_suggestion.is_none());
    }

    #[test]
    fn test_validation_error_warning() {
        let err = ValidationError::warning("Test warning");
        assert_eq!(err.severity, Severity::Warning);
        assert_eq!(err.message, "Test warning");
        assert!(err.fix_suggestion.is_none());
    }

    #[test]
    fn test_validation_error_info() {
        let err = ValidationError::info("Test info");
        assert_eq!(err.severity, Severity::Info);
        assert_eq!(err.message, "Test info");
        assert!(err.fix_suggestion.is_none());
    }

    #[test]
    fn test_validation_error_with_fix() {
        let err = ValidationError::error("Error").with_fix("Run this command");
        assert_eq!(err.fix_suggestion, Some("Run this command".to_string()));
    }

    #[test]
    fn test_validation_error_with_fix_chaining() {
        let err = ValidationError::warning("Warning message").with_fix("Fix suggestion");
        assert_eq!(err.severity, Severity::Warning);
        assert_eq!(err.message, "Warning message");
        assert_eq!(err.fix_suggestion, Some("Fix suggestion".to_string()));
    }

    #[test]
    fn test_validation_error_clone() {
        let err = ValidationError::error("Cloneable").with_fix("Fix");
        let cloned = err.clone();
        assert_eq!(cloned.severity, err.severity);
        assert_eq!(cloned.message, err.message);
        assert_eq!(cloned.fix_suggestion, err.fix_suggestion);
    }

    #[test]
    fn test_validation_error_debug() {
        let err = ValidationError::info("Debug test");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("ValidationError"));
        assert!(debug_str.contains("Debug test"));
    }

    #[test]
    fn test_validate_json_file_nonexistent() {
        let errors = validate_json_file(Path::new("/nonexistent/file.json"), "Test file");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_json_file_valid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("valid.json");
        fs::write(&file_path, r#"{"key": "value", "number": 42}"#).unwrap();

        let errors = validate_json_file(&file_path, "Valid JSON");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_json_file_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid.json");
        fs::write(&file_path, "{ invalid json }").unwrap();

        let errors = validate_json_file(&file_path, "Invalid JSON");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity, Severity::Error);
        assert!(errors[0].message.contains("Invalid JSON"));
    }

    #[test]
    fn test_validate_json_file_empty_object() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.json");
        fs::write(&file_path, "{}").unwrap();

        let errors = validate_json_file(&file_path, "Empty object");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_json_file_array() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("array.json");
        fs::write(&file_path, r#"[1, 2, 3, "four"]"#).unwrap();

        let errors = validate_json_file(&file_path, "Array");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validation_report_new() {
        let report = ValidationReport::new();
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 0);
    }

    #[test]
    fn test_validation_report_default() {
        let report = ValidationReport::default();
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 0);
    }

    #[test]
    fn test_validation_report_add_result_empty() {
        let mut report = ValidationReport::new();
        report.add_result("Test Validator", vec![]);
        assert_eq!(report.error_count(), 0);
    }

    #[test]
    fn test_validation_report_add_result_with_errors() {
        let mut report = ValidationReport::new();
        report.add_result(
            "Test Validator",
            vec![
                ValidationError::error("Error 1"),
                ValidationError::error("Error 2"),
            ],
        );
        assert_eq!(report.error_count(), 2);
    }

    #[test]
    fn test_validation_report_add_result_with_warnings() {
        let mut report = ValidationReport::new();
        report.add_result(
            "Test Validator",
            vec![
                ValidationError::warning("Warning 1"),
                ValidationError::warning("Warning 2"),
                ValidationError::warning("Warning 3"),
            ],
        );
        assert_eq!(report.warning_count(), 3);
    }

    #[test]
    fn test_validation_report_mixed_severities() {
        let mut report = ValidationReport::new();
        report.add_result(
            "Validator 1",
            vec![
                ValidationError::error("Error"),
                ValidationError::warning("Warning"),
                ValidationError::info("Info"),
            ],
        );
        report.add_result("Validator 2", vec![ValidationError::error("Another error")]);

        assert_eq!(report.error_count(), 2);
        assert_eq!(report.warning_count(), 1);
    }

    #[test]
    fn test_validation_report_print_does_not_panic() {
        let mut report = ValidationReport::new();
        report.add_result("Empty Validator", vec![]);
        report.add_result(
            "Error Validator",
            vec![ValidationError::error("Test error").with_fix("Fix it")],
        );
        report.add_result(
            "Warning Validator",
            vec![ValidationError::warning("Test warning")],
        );
        report.add_result("Info Validator", vec![ValidationError::info("Test info")]);

        // This should not panic
        report.print();
    }

    #[test]
    fn test_validate_task_name() {
        let profile = ActiveProfile::common_only();
        let task = ValidateTask::new(profile);
        assert_eq!(task.name(), "Validate");
    }

    #[test]
    fn test_validate_task_dry_run() {
        let profile = ActiveProfile::common_only();
        let task = ValidateTask::new(profile);
        let ops = task.dry_run();

        assert_eq!(ops.len(), 4);
        assert!(ops.iter().any(|op| op.description.contains("registry")));
        assert!(ops.iter().any(|op| op.description.contains("layer")));
        assert!(ops.iter().any(|op| op.description.contains("JSON")));
        assert!(ops.iter().any(|op| op.description.contains("legacy")));
    }

    #[test]
    fn test_config_validator_new() {
        let profile = ActiveProfile::common_only();
        let validator = ConfigValidator::new(profile);
        // Should have 4 validators
        assert_eq!(validator.validators.len(), 4);
    }

    #[test]
    fn test_config_validator_run_all() {
        let profile = ActiveProfile::with_profile("test-nonexistent");
        let validator = ConfigValidator::new(profile);
        let report = validator.run_all();

        // Should not panic, may have errors depending on environment
        report.print();
    }

    #[test]
    fn test_json_config_validator_name() {
        let profile = ActiveProfile::common_only();
        let validator = JsonConfigValidator::new(profile);
        assert_eq!(validator.name(), "JSON Configuration Files");
    }

    #[test]
    fn test_legacy_symlink_validator_name() {
        let validator = LegacySymlinkValidator::new();
        assert_eq!(validator.name(), "Legacy Symlink Check");
    }

    #[test]
    fn test_layer_validator_name() {
        let profile = ActiveProfile::common_only();
        let validator = LayerValidator::new(profile);
        assert_eq!(validator.name(), "Layer Resolution");
    }

    #[test]
    fn test_registry_validator_name() {
        let validator = RegistryValidator;
        assert_eq!(validator.name(), "Registry Files");
    }
}
