use crate::utils::display::{green, red, yellow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

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

pub trait Validator {
    fn name(&self) -> &str;
    fn validate(&self) -> Vec<ValidationError>;
}

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
                    let line = match error.severity {
                        Severity::Error => red(&format!(" x {}", error.message)),
                        Severity::Warning => yellow(&format!(" ! {}", error.message)),
                        Severity::Info => green(&format!(" i {}", error.message)),
                    };
                    println!("{}", line);
                    if let Some(fix) = &error.fix_suggestion {
                        println!("{}", yellow(&format!(" Fix: {}", fix)));
                    }
                }
            }
        }
    }
}
