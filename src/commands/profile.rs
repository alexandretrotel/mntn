use crate::cli::{ProfileActions, ProfileArgs};
use crate::profiles::{ProfileConfig, get_active_profile_name};
use crate::utils::paths::{get_profiles_config_path, get_profiles_path};
use std::fs;

pub fn run(args: ProfileArgs) {
    match args.action {
        Some(ProfileActions::List) => list_profiles(),
        Some(ProfileActions::Create { name, description }) => create_profile(&name, description),
        Some(ProfileActions::Delete { name }) => delete_profile(&name),
        None => {
            show_current_profile();
        }
    }
}

fn show_current_profile() {
    let current = get_active_profile_name();
    match current {
        Some(name) => println!("Active profile: {}", name),
        None => println!("No active profile (using common only)"),
    }
    println!();
    list_profiles();
    println!();
    println!("Use 'mntn use <profile>' to switch profiles");
}

fn list_profiles() {
    let config = ProfileConfig::load_or_default();
    let profiles = config.list_profiles();
    let current = get_active_profile_name();

    if profiles.is_empty() {
        println!("No profiles configured");
        println!();
        println!("Create a profile with: mntn profile create <name>");
        return;
    }

    println!("Available profiles:");
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

fn delete_profile(name: &str) {
    let path = get_profiles_config_path();
    let mut config = ProfileConfig::load_or_default();

    if !config.profile_exists(name) {
        eprintln!("Profile '{}' does not exist", name);
        return;
    }

    if let Some(current) = get_active_profile_name()
        && current == name
    {
        eprintln!(
            "Cannot delete active profile '{}'. Switch to another profile first.",
            name
        );
        return;
    }

    config.delete_profile(name);
    if let Err(e) = config.save(&path) {
        eprintln!("Failed to save profile config: {}", e);
        return;
    }

    let profile_dir = get_profiles_path(name);
    if profile_dir.exists() {
        println!("Profile directory exists at {}", profile_dir.display());
        println!("The directory was NOT deleted. Remove manually if desired:");
        println!("rm -rf {}", profile_dir.display());
    }

    println!("Deleted profile '{}'", name);
}
