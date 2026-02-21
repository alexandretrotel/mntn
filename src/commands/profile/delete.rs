use crate::profiles::{ProfileConfig, get_active_profile_name};
use crate::utils::paths::{get_profiles_config_path, get_profiles_path};

pub fn delete_profile(name: &str) {
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
