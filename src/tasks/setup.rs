use crate::logger::{log_error, log_info, log_success};
use crate::profile::{ActiveProfile, ProfileConfig};
use crate::utils::paths::{
    get_backup_root, get_mntn_dir, get_profile_config_path, set_active_profile,
};
use inquire::{Confirm, Text, error::InquireError};
use signal_hook::consts::SIGINT;
use signal_hook::flag;
use std::fs;
use std::process;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

fn prompt_or_abort<T, F: FnOnce() -> Result<T, InquireError>>(f: F) -> T {
    match f() {
        Ok(val) => val,
        Err(InquireError::OperationCanceled) => {
            println!("\nSetup aborted by user (Ctrl+C).");
            process::exit(130);
        }
        Err(_) => {
            println!("\nPrompt failed. Aborting setup.");
            process::exit(1);
        }
    }
}

pub fn run() {
    // Setup SIGINT (Ctrl+C) handler
    let running = Arc::new(AtomicBool::new(true));
    flag::register(SIGINT, Arc::clone(&running)).expect("Failed to register SIGINT handler");

    println!();
    println!("Welcome to mntn interactive setup!");
    println!("   This wizard will help you configure your dotfiles management.");
    println!();

    if let Err(e) = fs::create_dir_all(get_mntn_dir()) {
        log_error("Failed to create ~/.mntn directory", e);
        return;
    }

    let profile_name = prompt_or_abort(setup_profile_prompt);

    let should_migrate = prompt_or_abort(check_and_offer_migration_prompt);

    let should_backup = prompt_or_abort(|| {
        Confirm::new("Run initial backup now?")
            .with_default(true)
            .with_help_message("This will backup your current configurations")
            .prompt()
    });

    let include_encrypted = if should_backup {
        prompt_or_abort(|| {
            Confirm::new("Include encrypted configs in initial backup? (will prompt for a password)")
                .with_default(false)
                .with_help_message("Encrypted configs are skipped by default during setup to avoid prompting for a password. Choose yes to include them now.")
                .prompt()
        })
    } else {
        false
    };

    let should_install_tasks = prompt_or_abort(|| {
        Confirm::new("Install scheduled backup tasks?")
            .with_default(false)
            .with_help_message("This will set up automatic hourly backups")
            .prompt()
    });

    println!();
    println!("📋 Setup Summary:");
    if let Some(ref name) = profile_name {
        println!("   Profile: {}", name);
    } else {
        println!("   Profile: common (no active profile)");
    }
    if should_migrate {
        println!("   ✓ Migrate legacy files to common/");
    }
    if should_backup {
        println!("   ✓ Run initial backup");
        if include_encrypted {
            println!("   ✓ Include encrypted configs in initial backup");
        } else {
            println!("   ⚠ Encrypted configs will be skipped during initial backup");
        }
    }
    if should_install_tasks {
        println!("   ✓ Install scheduled tasks");
    }
    println!();

    let proceed = prompt_or_abort(|| {
        Confirm::new("Proceed with setup?")
            .with_default(true)
            .prompt()
    });

    if !proceed {
        log_info("Setup cancelled");
        return;
    }

    println!();

    // Save profile if created
    if let Some(ref name) = profile_name {
        save_profile_config(name);
        if let Err(e) = set_active_profile(name) {
            log_error("Failed to set active profile", e);
        }
    }

    if should_migrate {
        run_migration();
    }

    if should_backup {
        run_backup(&profile_name, include_encrypted);
    }

    if should_install_tasks {
        run_install_tasks();
    }

    println!();
    log_success("Setup complete!");
    println!();
    println!("Quick reference:");
    println!("   mntn backup          - Backup your configurations");
    println!("   mntn restore         - Restore configurations from backup");
    println!("   mntn use <name>      - Switch to a different profile");
    println!("   mntn profile         - List and manage profiles");
    println!("   mntn validate        - Check configuration status");
    println!("   mntn sync --help     - Git sync options");
    println!();
    println!("   Remember: Run 'mntn backup' after editing config files!");
    println!();
}

fn setup_profile_prompt() -> Result<Option<String>, inquire::error::InquireError> {
    println!("Profile Setup");
    println!("   Profiles let you maintain different configurations for different contexts");
    println!("   (e.g., 'work', 'personal', 'minimal', 'gaming')");
    println!();

    let create_profile = Confirm::new("Create a profile now?")
        .with_default(true)
        .with_help_message("You can always create profiles later with 'mntn profile create <name>'")
        .prompt()?;

    if !create_profile {
        println!("   ✓ Using common configuration only (no profile)");
        return Ok(None);
    }

    let profile_name = Text::new("Profile name:")
        .with_default("default")
        .with_help_message("e.g., work, personal, minimal, gaming")
        .with_validator(|input: &str| {
            if input.is_empty() {
                return Ok(inquire::validator::Validation::Invalid(
                    "Profile name cannot be empty".into(),
                ));
            }
            if input
                .chars()
                .any(|c| !c.is_alphanumeric() && c != '-' && c != '_')
            {
                return Ok(inquire::validator::Validation::Invalid(
                    "Use only letters, numbers, hyphens, and underscores".into(),
                ));
            }
            Ok(inquire::validator::Validation::Valid)
        })
        .prompt()?;

    println!("   ✓ Profile: {}", profile_name);
    Ok(Some(profile_name))
}

fn save_profile_config(profile_name: &str) {
    let path = get_profile_config_path();

    let mut config = ProfileConfig::load(&path).unwrap_or_default();

    if config.version.is_empty() {
        config.version = "1.0.0".to_string();
    }

    config.create_profile(
        profile_name,
        Some("Profile created during setup".to_string()),
    );

    if let Err(e) = config.save(&path) {
        log_error("Failed to save profile config", Box::new(e));
    }
}

fn check_and_offer_migration_prompt() -> Result<bool, inquire::error::InquireError> {
    let backup_root = get_backup_root();

    if !backup_root.exists() {
        return Ok(false);
    }

    let has_legacy_files = fs::read_dir(&backup_root)
        .map(|entries| {
            entries.filter_map(Result::ok).any(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                name_str != "common"
                    && name_str != "profiles"
                    && name_str != "packages"
                    && !name_str.ends_with(".txt")
            })
        })
        .unwrap_or(false);

    if !has_legacy_files {
        return Ok(false);
    }

    println!();
    println!("Legacy Files Detected");
    println!("   Found files in ~/.mntn/backup/ that aren't in the layered structure.");

    Confirm::new("Migrate legacy files to common/ layer?")
        .with_default(true)
        .with_help_message("This moves existing configs to the shared 'common' layer")
        .prompt()
}

fn run_migration() {
    println!("Migrating legacy files to common/...");

    let mut task = crate::tasks::migrate::MigrateTask::new();
    if let Err(e) = crate::tasks::core::Task::execute(&mut task) {
        log_error("Error during migration", e);
    }
}

fn run_backup(profile_name: &Option<String>, include_encrypted: bool) {
    println!("Running initial backup...");

    let profile = match profile_name {
        Some(name) => ActiveProfile::with_profile(name),
        None => ActiveProfile::common_only(),
    };

    let skip_encrypted = !include_encrypted;
    if include_encrypted {
        println!("   Note: Encrypted configs will be included and you will be prompted for a password if needed.");
    } else {
        println!("   Note: Encrypted configs are skipped to avoid prompting for a password during initial setup.");
    }

    let mut task = crate::tasks::backup::BackupTask::new(profile, skip_encrypted);
    if let Err(e) = crate::tasks::core::Task::execute(&mut task) {
        log_error("Error during backup", e);
    }
}

fn run_install_tasks() {
    println!("Installing scheduled tasks...");

    let mut task = crate::tasks::install::InstallTask::new(false);
    if let Err(e) = crate::tasks::core::Task::execute(&mut task) {
        log_error("Error during install", e);
    }
}
