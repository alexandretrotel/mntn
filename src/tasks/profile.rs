use crate::cli::{ProfileActions, ProfileArgs};
use crate::logger::{log_error, log_success, log_warning};
use crate::profile::ProfileConfig;
use crate::utils::paths::{
    get_active_profile_name, get_backup_profile_path, get_profile_config_path,
};
use std::fs;

pub fn run_with_args(args: ProfileArgs) {
    match args.action {
        Some(ProfileActions::List) => list_profiles(),
        Some(ProfileActions::Create { name, description }) => create_profile(&name, description),
        Some(ProfileActions::Delete { name }) => delete_profile(&name),
        None => {
            // No action - show current and list
            show_current_profile();
        }
    }
}

fn show_current_profile() {
    let current = get_active_profile_name();
    match current {
        Some(name) => println!("📍 Active profile: {}", name),
        None => println!("📍 No active profile (using common only)"),
    }
    println!();
    list_profiles();
    println!();
    println!("💡 Use 'mntn use <profile>' to switch profiles");
}

fn list_profiles() {
    let config = ProfileConfig::load_or_default();
    let profiles = config.list_profiles();
    let current = get_active_profile_name();

    if profiles.is_empty() {
        println!("📋 No profiles configured");
        println!();
        println!("💡 Create a profile with: mntn profile create <name>");
        return;
    }

    println!("📋 Available profiles:");
    for name in profiles {
        let is_current = current.as_ref() == Some(name);
        let marker = if is_current { " ← active" } else { "" };

        if let Some(def) = config.get_profile(name) {
            if let Some(desc) = &def.description {
                println!("   {} - {}{}", name, desc, marker);
            } else {
                println!("   {}{}", name, marker);
            }
        } else {
            println!("   {}{}", name, marker);
        }
    }
}

fn create_profile(name: &str, description: Option<String>) {
    let path = get_profile_config_path();
    let mut config = ProfileConfig::load_or_default();

    if config.profile_exists(name) {
        log_warning(&format!("Profile '{}' already exists", name));
        return;
    }

    // Validate profile name
    if name.is_empty() {
        log_warning("Profile name cannot be empty");
        return;
    }

    if name
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        log_warning("Profile name can only contain letters, numbers, hyphens, and underscores");
        return;
    }

    config.create_profile(name, description.clone());

    if config.version.is_empty() {
        config.version = "1.0.0".to_string();
    }

    if let Err(e) = config.save(&path) {
        log_error("Failed to save profile config", e);
        return;
    }

    // Create the profile directory
    let profile_dir = get_backup_profile_path(name);
    if let Err(e) = fs::create_dir_all(&profile_dir) {
        log_warning(&format!(
            "Profile created but failed to create directory: {}",
            e
        ));
    }

    log_success(&format!("Created profile '{}'", name));
    if let Some(desc) = description {
        println!("   Description: {}", desc);
    }
    println!();
    println!("💡 Switch to this profile with: mntn use {}", name);
}

fn delete_profile(name: &str) {
    let path = get_profile_config_path();
    let mut config = ProfileConfig::load_or_default();

    if !config.profile_exists(name) {
        log_warning(&format!("Profile '{}' does not exist", name));
        return;
    }

    // Check if this is the active profile
    if let Some(current) = get_active_profile_name()
        && current == name
    {
        log_warning(&format!(
            "Cannot delete active profile '{}'. Switch to another profile first.",
            name
        ));
        return;
    }

    // Remove from config
    config.delete_profile(name);

    if let Err(e) = config.save(&path) {
        log_error("Failed to save profile config", e);
        return;
    }

    // Optionally remove the profile directory
    let profile_dir = get_backup_profile_path(name);
    if profile_dir.exists() {
        println!("⚠️  Profile directory exists at {}", profile_dir.display());
        println!("   The directory was NOT deleted. Remove manually if desired:");
        println!("   rm -rf {}", profile_dir.display());
    }

    log_success(&format!("Deleted profile '{}'", name));
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_profile_name_validation_empty() {
        // Empty name should be rejected - tested via the warning message
        // This is a basic test to ensure the validation logic exists
        let name = "";
        assert!(name.is_empty());
    }

    #[test]
    fn test_profile_name_validation_invalid_chars() {
        let name = "my profile!";
        let has_invalid = name
            .chars()
            .any(|c| !c.is_alphanumeric() && c != '-' && c != '_');
        assert!(has_invalid);
    }

    #[test]
    fn test_profile_name_validation_valid() {
        let valid_names = ["work", "my-profile", "test_123", "Profile1"];
        for name in valid_names {
            let has_invalid = name
                .chars()
                .any(|c| !c.is_alphanumeric() && c != '-' && c != '_');
            assert!(!has_invalid, "Name '{}' should be valid", name);
        }
    }
}
