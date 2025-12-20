use crate::cli::UseArgs;
use crate::logger::{log_error, log_info, log_success, log_warning};
use crate::profile::ProfileConfig;
use crate::utils::paths::{clear_active_profile, set_active_profile};

pub fn run_with_args(args: UseArgs) {
    switch_to_profile(&args.profile);
}

fn switch_to_profile(name: &str) {
    let config = ProfileConfig::load_or_default();

    // Allow switching to "common" or "none" to clear active profile
    if name == "common" || name == "none" {
        if let Err(e) = clear_active_profile() {
            log_error("Failed to clear active profile", e);
            return;
        }
        log_success("Switched to common (no active profile)");
        return;
    }

    // Check if profile exists
    if !config.profile_exists(name) {
        log_warning(&format!("Profile '{}' does not exist", name));
        println!();
        println!("💡 Create it with: mntn profile create {}", name);
        println!("   Or list available profiles: mntn profile list");
        return;
    }

    // Set as active profile
    if let Err(e) = set_active_profile(name) {
        log_error("Failed to set active profile", e);
        return;
    }

    log_success(&format!("Switched to profile '{}'", name));

    // Ask if user wants to restore
    println!();
    log_info("Run 'mntn restore' to apply this profile's configurations");
}
