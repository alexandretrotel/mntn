use crate::logger::{log_error, log_info, log_success, log_warning};
use crate::profile::{ActiveProfile, ProfileConfig, ProfileDefinition};
use crate::tasks::migrate::MigrateTarget;
use crate::utils::paths::{
    get_backup_root, get_machine_id_path, get_machine_identifier, get_mntn_dir,
    get_profile_config_path,
};
use inquire::{Confirm, Select, Text};
use std::fs;

pub fn run() {
    println!();
    println!("🚀 Welcome to mntn interactive setup!");
    println!("   This wizard will help you configure your dotfiles management.");
    println!();

    if let Err(e) = fs::create_dir_all(get_mntn_dir()) {
        log_error("Failed to create ~/.mntn directory", e);
        return;
    }

    let machine_id = setup_machine_id();
    let environment = setup_environment();

    save_profile_config(&machine_id, &environment);

    let should_migrate = check_and_offer_migration();

    let should_backup = Confirm::new("Run initial backup now?")
        .with_default(true)
        .with_help_message("This will backup your current configurations")
        .prompt()
        .unwrap_or(false);

    let should_link = Confirm::new("Create symlinks for your configurations?")
        .with_default(true)
        .with_help_message("This will link your dotfiles to the backup location")
        .prompt()
        .unwrap_or(false);

    let should_install_tasks = Confirm::new("Install scheduled backup tasks?")
        .with_default(false)
        .with_help_message("This will set up automatic hourly backups")
        .prompt()
        .unwrap_or(false);

    println!();
    println!("📋 Setup Summary:");
    println!("   Machine ID: {}", machine_id);
    println!("   Environment: {}", environment);
    if should_migrate {
        println!("   ✓ Migrate legacy files to common/");
    }
    if should_backup {
        println!("   ✓ Run initial backup");
    }
    if should_link {
        println!("   ✓ Create symlinks");
    }
    if should_install_tasks {
        println!("   ✓ Install scheduled tasks");
    }
    println!();

    let proceed = Confirm::new("Proceed with setup?")
        .with_default(true)
        .prompt()
        .unwrap_or(false);

    if !proceed {
        log_info("Setup cancelled");
        return;
    }

    println!();

    if should_migrate {
        run_migration(&machine_id, &environment);
    }

    if should_backup {
        run_backup(&machine_id, &environment);
    }

    if should_link {
        run_link(&machine_id, &environment);
    }

    if should_install_tasks {
        run_install_tasks();
    }

    println!();
    log_success("Setup complete!");
    println!();
    println!("📖 Quick reference:");
    println!("   mntn backup          - Backup your configurations");
    println!("   mntn link            - Create/update symlinks");
    println!("   mntn validate        - Check configuration status");
    println!("   mntn migrate         - Move files between layers");
    println!("   mntn sync --help     - Git sync options");
    println!();
    println!("   Use --profile, --env, or --machine-id flags to override defaults.");
    println!();
}

fn setup_machine_id() -> String {
    let current = get_machine_identifier();
    let machine_id_path = get_machine_id_path();

    let has_custom_id = machine_id_path.exists();

    println!("📍 Machine Identifier");
    if has_custom_id {
        println!("   Current: {} (from ~/.mntn/.machine-id)", current);
    } else {
        println!("   Auto-detected: {}", current);
    }

    let use_custom = Confirm::new("Set a custom machine identifier?")
        .with_default(false)
        .with_help_message("Useful for identifying this machine in your dotfiles")
        .prompt()
        .unwrap_or(false);

    if use_custom {
        let custom_id = Text::new("Enter machine identifier:")
            .with_default(&current)
            .with_help_message("e.g., work-laptop, home-desktop, macbook-pro")
            .prompt()
            .unwrap_or_else(|_| current.clone());

        if let Err(e) = fs::write(&machine_id_path, &custom_id) {
            log_warning(&format!("Failed to save machine ID: {}", e));
        } else {
            println!("   ✓ Saved machine ID: {}", custom_id);
        }
        return custom_id;
    }

    current
}

fn setup_environment() -> String {
    println!();
    println!("🌍 Environment");

    let environments = vec!["default", "work", "personal", "dev", "custom..."];

    let selection = Select::new("Select your environment:", environments)
        .with_help_message("Environment determines which config layer to use")
        .prompt()
        .unwrap_or("default");

    let environment = if selection == "custom..." {
        Text::new("Enter custom environment name:")
            .with_default("default")
            .prompt()
            .unwrap_or_else(|_| "default".to_string())
    } else {
        selection.to_string()
    };

    println!("   ✓ Environment: {}", environment);
    environment
}

fn save_profile_config(machine_id: &str, environment: &str) {
    let path = get_profile_config_path();

    let mut config = ProfileConfig::load(&path).unwrap_or_default();

    if config.version.is_empty() {
        config.version = "1.0.0".to_string();
    }

    let profile_name = format!("{}-{}", machine_id, environment);
    config.profiles.insert(
        profile_name.clone(),
        ProfileDefinition {
            machine_id: Some(machine_id.to_string()),
            environment: Some(environment.to_string()),
            description: Some(format!(
                "Auto-generated profile for {} in {} environment",
                machine_id, environment
            )),
        },
    );

    if config.default_profile.is_none() {
        config.default_profile = Some(profile_name);
    }

    if let Err(e) = config.save(&path) {
        log_warning(&format!("Failed to save profile config: {}", e));
    }
}

fn check_and_offer_migration() -> bool {
    let backup_root = get_backup_root();

    if !backup_root.exists() {
        return false;
    }

    let has_legacy_files = fs::read_dir(&backup_root)
        .map(|entries| {
            entries.filter_map(Result::ok).any(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                name_str != "common"
                    && name_str != "machines"
                    && name_str != "environments"
                    && !name_str.ends_with(".txt")
            })
        })
        .unwrap_or(false);

    if !has_legacy_files {
        return false;
    }

    println!();
    println!("📁 Legacy Files Detected");
    println!("   Found files in ~/.mntn/backup/ that aren't in the layered structure.");

    Confirm::new("Migrate legacy files to common/ layer?")
        .with_default(true)
        .with_help_message("This moves existing configs to the shared 'common' layer")
        .prompt()
        .unwrap_or(false)
}

fn run_migration(machine_id: &str, environment: &str) {
    println!("🔄 Migrating legacy files to common/...");

    let profile = ActiveProfile::resolve(None, Some(machine_id), Some(environment));

    let mut task = crate::tasks::migrate::MigrateTask::new(profile, MigrateTarget::Common);
    if let Err(e) = crate::tasks::core::Task::execute(&mut task) {
        log_error("Error during migration", e);
    }
}

fn run_backup(machine_id: &str, environment: &str) {
    println!("🔁 Running initial backup...");

    let profile = ActiveProfile::resolve(None, Some(machine_id), Some(environment));

    let mut task = crate::tasks::backup::BackupTask::new(profile, MigrateTarget::Common);
    if let Err(e) = crate::tasks::core::Task::execute(&mut task) {
        log_error("Error during backup", e);
    }
}

fn run_link(machine_id: &str, environment: &str) {
    println!("🔗 Creating symlinks...");

    let profile = ActiveProfile::resolve(None, Some(machine_id), Some(environment));

    let mut task = crate::tasks::link::LinkTask::new(profile);
    if let Err(e) = crate::tasks::core::Task::execute(&mut task) {
        log_error("Error during link", e);
    }
}

fn run_install_tasks() {
    println!("📦 Installing scheduled tasks...");

    let mut task = crate::tasks::install::InstallTask::new(false);
    if let Err(e) = crate::tasks::core::Task::execute(&mut task) {
        log_error("Error during install", e);
    }
}
