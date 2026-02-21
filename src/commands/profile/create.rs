use crate::profiles::ProfileConfig;
use crate::utils::paths::{get_profiles_config_path, get_profiles_path};
use std::fs;

pub fn create_profile(name: &str, description: Option<String>) {
    let path = get_profiles_config_path();
    let mut config = ProfileConfig::load_or_default();

    if config.profile_exists(name) {
        eprintln!("Profile '{}' already exists", name);
        return;
    }

    if name.is_empty() {
        eprintln!("Profile name cannot be empty");
        return;
    }

    if name
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        eprintln!("Profile name can only contain letters, numbers, hyphens, and underscores");
        return;
    }

    config.create_profile(name, description.clone());
    if config.version.is_empty() {
        config.version = "1.0.0".to_string();
    }

    if let Err(e) = config.save(&path) {
        eprintln!("Failed to save profile config: {}", e);
        return;
    }

    let profile_dir = get_profiles_path(name);
    if let Err(e) = fs::create_dir_all(&profile_dir) {
        eprintln!("Profile created but failed to create directory: {}", e);
    }

    println!("Created profile '{}'", name);
    if let Some(desc) = description {
        println!("   Description: {}", desc);
    }
    println!();
    println!("Switch to this profile with: mntn use {}", name);
}
