use crate::commands::core::Command;
use crate::encryption::{encrypt_file, get_encrypted_path, prompt_password};
use crate::profiles::ActiveProfile;
use crate::registry::config::ConfigRegistry;
use crate::registry::encrypted::EncryptedRegistry;
use crate::registry::package::PackageRegistry;
use crate::utils::paths::{
    get_config_registry_path, get_encrypted_registry_path, get_mntn_dir, get_package_registry_path,
    get_packages_path,
};
use crate::utils::system::{run_cmd, strip_ansi_codes, sync_directory_contents};
use age::secrecy::SecretString;
use rayon::prelude::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct BackupTask {
    profile: ActiveProfile,
    skip_encrypted: bool,
}

impl BackupTask {
    pub fn new(profile: ActiveProfile, skip_encrypted: bool) -> Self {
        Self {
            profile,
            skip_encrypted,
        }
    }
}

impl Command for BackupTask {
    fn name(&self) -> &str {
        "Backup"
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let mntn_dir = get_mntn_dir();
        let backup_path = self.profile.get_backup_path();
        fs::create_dir_all(&backup_path)?;

        if !mntn_dir.exists() {
            crate::commands::git::init_repo_if_missing(&mntn_dir)?;
        }

        println!("Backing up...");
        println!("   Target: {}", self.profile);

        let packages_path = get_packages_path();
        fs::create_dir_all(&packages_path)?;

        backup_configs(&backup_path);
        backup_packages(&packages_path);

        if !self.skip_encrypted {
            let encrypted_backup_path = self.profile.get_encrypted_backup_path();
            fs::create_dir_all(&encrypted_backup_path)?;

            match prompt_password(true) {
                Ok(password) => {
                    backup_encrypted_configs(&encrypted_backup_path, &password);
                }
                Err(e) => {
                    eprintln!("Skipping encrypted backup: {}", e);
                }
            }
        }

        println!("Backup complete");
        Ok(())
    }
}

pub fn run(args: crate::cli::BackupArgs) {
    use crate::commands::core::CommandExecutor;

    let profile = args.resolve_profile();
    CommandExecutor::run(&mut BackupTask::new(profile, args.skip_encrypted));
}

fn backup_packages(packages_path: &Path) {
    let package_registry_path = get_package_registry_path();
    let package_registry = match PackageRegistry::load_or_create(&package_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            eprintln!(
                "Failed to load package registry, skipping package backup: {}",
                e
            );
            return;
        }
    };

    let current_platform = PackageRegistry::get_current_platform();
    let compatible_entries: Vec<_> = package_registry
        .get_platform_compatible_entries(&current_platform)
        .collect();

    if compatible_entries.is_empty() {
        println!("No package managers found to backup");
        return;
    }

    println!(
        "Backing up {} package managers...",
        compatible_entries.len()
    );

    let results: Vec<_> = compatible_entries
        .par_iter()
        .map(|(id, entry)| {
            let args: Vec<&str> = entry.args.iter().map(|s| s.as_str()).collect();
            let result = match run_cmd(&entry.command, &args, None) {
                Ok(content) => Ok(content),
                Err(e) => Err(e.to_string()),
            };
            ((*id).clone(), (*entry).clone(), result)
        })
        .collect();

    for (id, entry, result) in results {
        match result {
            Ok(content) => {
                let content = strip_ansi_codes(&content);
                let output_path = packages_path.join(&entry.output_file);
                let tmp_path = output_path.with_extension("tmp");

                match fs::File::create(&tmp_path).and_then(|mut f| f.write_all(content.as_bytes()))
                {
                    Ok(_) => {
                        if let Err(e) = fs::rename(&tmp_path, &output_path) {
                            eprintln!("Failed to atomically move {}: {}", entry.output_file, e);
                        } else {
                            println!("Backed up {} ({})", entry.name, id);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to write temp file for {}: {}", entry.output_file, e);
                        let _ = fs::remove_file(&tmp_path);
                    }
                }
            }
            Err(e) => {
                eprintln!("Command for {} failed: {}", entry.name, e);
            }
        }
    }
}

fn backup_configs(configs_path: &Path) {
    let config_registry_path = get_config_registry_path();
    let config_registry = match ConfigRegistry::load_or_create(&config_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            eprintln!(
                "Failed to load registry, skipping config file backup: {}",
                e
            );
            return;
        }
    };

    let enabled_entries: Vec<_> = config_registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("No configuration files found to backup");
        return;
    }

    println!(
        "Backing up {} configuration files...",
        enabled_entries.len()
    );

    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;
        let backup_destination = configs_path.join(&entry.source_path);

        if let Some(parent) = backup_destination.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            eprintln!(
                "Failed to create backup directory for {}: {}",
                entry.name, e
            );
            continue;
        }

        let result = if target_path.is_dir() {
            backup_directory(target_path, &backup_destination)
        } else {
            backup_file(target_path, &backup_destination)
        };

        match result {
            Ok(()) => {
                println!("Backed up {} ({})", entry.name, id);
                println!("Backed up {} from {}", entry.name, target_path.display());
            }
            Err(e) => {
                eprintln!("Failed to backup {}: {}", entry.name, e);
            }
        }
    }
}

fn backup_file(source: &PathBuf, destination: &PathBuf) -> std::io::Result<()> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source file {} not found", source.display()),
        ));
    }

    if source.is_symlink()
        && let Ok(link_target) = fs::read_link(source)
    {
        let canonical_target = link_target.canonicalize().unwrap_or(link_target.clone());
        let canonical_dest = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.clone());

        if canonical_target == canonical_dest {
            let content = fs::read(&canonical_target)?;
            fs::remove_file(source)?;
            fs::write(source, &content)?;
            println!("Converted symlink to real file: {}", source.display());
            return Ok(());
        }
    }

    fs::copy(source, destination)?;
    Ok(())
}

fn backup_directory(source: &PathBuf, destination: &PathBuf) -> std::io::Result<()> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source directory {} not found", source.display()),
        ));
    }

    if source.is_symlink()
        && let Ok(link_target) = fs::read_link(source)
    {
        let canonical_target = link_target
            .canonicalize()
            .unwrap_or_else(|_| link_target.clone());
        let canonical_dest = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.clone());

        if canonical_target == canonical_dest
            || canonical_dest.starts_with(&canonical_target)
            || canonical_target.starts_with(&canonical_dest)
        {
            fs::remove_file(source)?;
            fs::create_dir_all(source)?;
            crate::utils::filesystem::copy_dir_recursive(&canonical_target, source)?;
            println!("Converted symlink to real directory: {}", source.display());
            return Ok(());
        }
    }

    fs::create_dir_all(destination)?;
    sync_directory_contents(source, destination)
}

fn backup_encrypted_configs(encrypted_backup_path: &Path, password: &SecretString) {
    let registry_path = get_encrypted_registry_path();
    let encrypted_registry = match EncryptedRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            eprintln!(
                "Failed to load encrypted registry, skipping encrypted backup: {}",
                e
            );
            return;
        }
    };

    let enabled_entries: Vec<_> = encrypted_registry.get_enabled_entries().collect();

    if enabled_entries.is_empty() {
        println!("No encrypted configuration files found to backup");
        return;
    }

    println!(
        "Backing up {} encrypted configuration files...",
        enabled_entries.len()
    );

    for (id, entry) in enabled_entries {
        let target_path = &entry.target_path;

        if !target_path.exists() {
            eprintln!(
                "Source file for {} not found: {}",
                entry.name,
                target_path.display()
            );
            continue;
        }

        let encrypted_path = get_encrypted_path(&entry.source_path);
        let backup_destination = encrypted_backup_path.join(&encrypted_path);

        if let Some(parent) = backup_destination.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            eprintln!(
                "Failed to create backup directory for {}: {}",
                entry.name, e
            );
            continue;
        }

        match encrypt_file(target_path, &backup_destination, password) {
            Ok(()) => {
                println!("Backed up {} ({})", entry.name, id);
                println!(
                    "Backed up encrypted {} from {}",
                    entry.name,
                    target_path.display()
                );
            }
            Err(e) => {
                eprintln!("Failed to backup encrypted {}: {}", entry.name, e);
            }
        }
    }
}
