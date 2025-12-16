use crate::logger::log;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::registries::package_registry::PackageRegistry;
use crate::utils::paths::{get_backup_path, get_package_registry_path, get_registry_path};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

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
pub struct JsonConfigValidator;

impl Validator for JsonConfigValidator {
    fn validate(&self) -> Vec<ValidationError> {
        let backup_dir = get_backup_path();
        let json_files = [
            ("vscode/settings.json", "VS Code settings"),
            ("vscode/keybindings.json", "VS Code keybindings"),
            ("zed/settings.json", "Zed settings"),
        ];
        json_files
            .iter()
            .flat_map(|&(file_path, description)| {
                let full_path = backup_dir.join(file_path);
                validate_json_file(&full_path, description)
            })
            .collect()
    }

    fn name(&self) -> &str {
        "JSON Configuration Files"
    }
}

/// Validates symlinks are correctly configured
pub struct SymlinkValidator;

impl Validator for SymlinkValidator {
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
        let backup_dir = get_backup_path();
        for (id, entry) in registry.get_enabled_entries() {
            let source = backup_dir.join(&entry.source_path);
            let target: PathBuf = entry.target_path.clone().into(); // Assuming target_path is String or impl Into<PathBuf>
            if !source.exists() {
                errors.push(
                    ValidationError::warning(format!(
                        "{} ({}): Source file missing in backup",
                        entry.name, id
                    ))
                    .with_fix(format!(
                        "Run 'mntn backup' or check if {} exists",
                        source.display()
                    )),
                );
                continue;
            }
            if target.is_symlink() {
                match fs::read_link(&target) {
                    Ok(link_target) => {
                        if link_target != source {
                            errors.push(
                                ValidationError::warning(format!(
                                    "{} ({}): Symlink points to wrong location",
                                    entry.name, id
                                ))
                                .with_fix(format!(
                                    "Run 'mntn link' to fix. Expected: {}, Found: {}",
                                    source.display(),
                                    link_target.display()
                                )),
                            );
                        }
                    }
                    Err(e) => {
                        errors.push(ValidationError::error(format!(
                            "{} ({}): Could not read symlink: {}",
                            entry.name, id, e
                        )));
                    }
                }
            } else if target.exists() {
                errors.push(
                    ValidationError::info(format!(
                        "{} ({}): Target exists but is not a symlink",
                        entry.name, id
                    ))
                    .with_fix(
                        "Run 'mntn link' to create symlink (existing file will be backed up)",
                    ),
                );
            } else {
                errors.push(
                    ValidationError::info(format!(
                        "{} ({}): Target does not exist",
                        entry.name, id
                    ))
                    .with_fix("Run 'mntn link' to create symlink"),
                );
            }
        }
        errors
    }

    fn name(&self) -> &str {
        "Symlink Configuration"
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
                                "Install {} or disable this entry with 'mntn package-registry toggle {} -e false'",
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
    pub fn new() -> Self {
        let validators: Vec<Box<dyn Validator>> = vec![
            Box::new(RegistryValidator),
            Box::new(JsonConfigValidator),
            Box::new(SymlinkValidator),
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

/// Run the validation command
pub fn run() {
    println!("Validating configuration...");
    log("Starting validation");
    let validator = ConfigValidator::new();
    let report = validator.run_all();
    println!();
    report.print();
    println!();
    let error_count = report.error_count();
    let warning_count = report.warning_count();
    if error_count == 0 && warning_count == 0 {
        println!("All checks passed.");
        log("Validation complete: all checks passed");
    } else {
        println!(
            "Validation complete: {} error(s), {} warning(s)",
            error_count, warning_count
        );
        log(&format!(
            "Validation complete: {} error(s), {} warning(s)",
            error_count, warning_count
        ));
    }
}
