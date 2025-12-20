use crate::cli::UseArgs;
use crate::logger::{log_error, log_info, log_success, log_warning};
use crate::profile::ProfileConfig;
use crate::tasks::core::{PlannedOperation, Task, TaskExecutor};
use crate::utils::paths::{clear_active_profile, get_active_profile_name, set_active_profile};

pub struct UseProfileTask {
    profile_name: String,
}

impl UseProfileTask {
    pub fn new(profile_name: String) -> Self {
        Self { profile_name }
    }

    fn is_clearing_profile(&self) -> bool {
        self.profile_name == "common" || self.profile_name == "none"
    }
}

impl Task for UseProfileTask {
    fn name(&self) -> &str {
        "Use Profile"
    }

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = ProfileConfig::load_or_default();

        // Allow switching to "common" or "none" to clear active profile
        if self.is_clearing_profile() {
            clear_active_profile()?;
            log_success("Switched to common (no active profile)");
            return Ok(());
        }

        // Check if profile exists
        if !config.profile_exists(&self.profile_name) {
            log_warning(&format!("Profile '{}' does not exist", self.profile_name));
            println!();
            println!(
                "💡 Create it with: mntn profile create {}",
                self.profile_name
            );
            println!("   Or list available profiles: mntn profile list");
            return Ok(());
        }

        // Set as active profile
        set_active_profile(&self.profile_name)?;

        log_success(&format!("Switched to profile '{}'", self.profile_name));

        // Suggest running restore
        println!();
        log_info("Run 'mntn restore' to apply this profile's configurations");

        Ok(())
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let config = ProfileConfig::load_or_default();
        let current = get_active_profile_name();

        if self.is_clearing_profile() {
            match current {
                Some(name) => {
                    operations.push(PlannedOperation::with_target(
                        format!("Clear active profile '{}'", name),
                        "common (no active profile)".to_string(),
                    ));
                }
                None => {
                    operations.push(PlannedOperation::new(
                        "Already using common (no active profile)",
                    ));
                }
            }
            return operations;
        }

        // Check if profile exists
        if !config.profile_exists(&self.profile_name) {
            operations.push(PlannedOperation::new(format!(
                "Profile '{}' does not exist",
                self.profile_name
            )));
            return operations;
        }

        // Check if already on this profile
        if current.as_deref() == Some(&self.profile_name) {
            operations.push(PlannedOperation::new(format!(
                "Already using profile '{}'",
                self.profile_name
            )));
            return operations;
        }

        // Show the switch operation
        let from = current.unwrap_or_else(|| "common".to_string());
        operations.push(PlannedOperation::with_target(
            format!("Switch from '{}'", from),
            format!("profile '{}'", self.profile_name),
        ));

        operations
    }
}

pub fn run_with_args(args: UseArgs) {
    let mut task = UseProfileTask::new(args.profile);

    if args.dry_run {
        TaskExecutor::run(&mut task, true);
    } else if let Err(e) = task.execute() {
        log_error("Failed to switch profile", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_profile_task_name() {
        let task = UseProfileTask::new("test".to_string());
        assert_eq!(task.name(), "Use Profile");
    }

    #[test]
    fn test_use_profile_task_is_clearing_common() {
        let task = UseProfileTask::new("common".to_string());
        assert!(task.is_clearing_profile());
    }

    #[test]
    fn test_use_profile_task_is_clearing_none() {
        let task = UseProfileTask::new("none".to_string());
        assert!(task.is_clearing_profile());
    }

    #[test]
    fn test_use_profile_task_is_not_clearing() {
        let task = UseProfileTask::new("work".to_string());
        assert!(!task.is_clearing_profile());
    }

    #[test]
    fn test_use_profile_task_dry_run_nonexistent() {
        let task = UseProfileTask::new("nonexistent-profile-12345".to_string());
        let ops = task.dry_run();
        assert!(!ops.is_empty());
        assert!(ops[0].description.contains("does not exist"));
    }

    #[test]
    fn test_use_profile_task_dry_run_clearing() {
        let task = UseProfileTask::new("common".to_string());
        let ops = task.dry_run();
        assert!(!ops.is_empty());
    }
}
