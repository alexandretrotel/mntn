use crate::commands::core::{Command, CommandExecutor};
use crate::profiles::{ActiveProfile, ProfileConfig};

mod types;
mod utils;
mod validators;

use validators::ValidationSuite;

struct ValidateTask {
    profile: ActiveProfile,
    skip_encrypted: bool,
}

impl ValidateTask {
    fn new(profile: ActiveProfile, skip_encrypted: bool) -> Self {
        Self {
            profile,
            skip_encrypted,
        }
    }
}

impl Command for ValidateTask {
    fn name(&self) -> &str {
        "Validate"
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        println!("Validating configuration...");
        println!("   Profile: {}", self.profile);
        println!("Starting validation");

        let validator = ValidationSuite::new(self.profile.clone(), self.skip_encrypted);
        let report = validator.run_all();
        println!();
        report.print();
        println!();
        let error_count = report.error_count();
        let warning_count = report.warning_count();
        if error_count == 0 && warning_count == 0 {
            println!("All checks passed");
        } else {
            eprintln!(
                "Validation complete: {} error(s), {} warning(s)",
                error_count, warning_count
            );
        }
        Ok(())
    }
}

pub(crate) fn run(args: crate::cli::ValidateArgs) {
    if let Ok(true) = ProfileConfig::save_default_if_missing() {
        println!("Created default profile config at ~/.mntn/profiles.json");
    }

    let profile = args.resolve_profile();
    CommandExecutor::run(&mut ValidateTask::new(profile, args.skip_encrypted));
}
