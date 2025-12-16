use crate::logger::log;
use crate::profile::ActiveProfile;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::tasks::core::{PlannedOperation, Task};
use crate::utils::paths::{
    get_backup_common_path, get_backup_environment_path, get_backup_machine_path, get_backup_root,
    get_registry_path,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrateTarget {
    Common,
    Machine,
    Environment,
}

impl std::fmt::Display for MigrateTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateTarget::Common => write!(f, "common"),
            MigrateTarget::Machine => write!(f, "machine"),
            MigrateTarget::Environment => write!(f, "environment"),
        }
    }
}

pub struct MigrateTask {
    profile: ActiveProfile,
    target: MigrateTarget,
}

impl MigrateTask {
    pub fn new(profile: ActiveProfile, target: MigrateTarget) -> Self {
        Self { profile, target }
    }

    fn get_target_dir(&self) -> PathBuf {
        match self.target {
            MigrateTarget::Common => get_backup_common_path(),
            MigrateTarget::Machine => get_backup_machine_path(&self.profile.machine_id),
            MigrateTarget::Environment => get_backup_environment_path(&self.profile.environment),
        }
    }

    fn find_legacy_files(&self) -> Vec<(String, PathBuf)> {
        let registry_path = get_registry_path();
        let registry = match ConfigsRegistry::load_or_create(&registry_path) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        let mut legacy_files = Vec::new();
        let backup_root = get_backup_root();

        for (_id, entry) in registry.get_enabled_entries() {
            let legacy_path = backup_root.join(&entry.source_path);

            if legacy_path.exists() {
                let common_path = get_backup_common_path().join(&entry.source_path);
                let machine_path =
                    get_backup_machine_path(&self.profile.machine_id).join(&entry.source_path);
                let env_path =
                    get_backup_environment_path(&self.profile.environment).join(&entry.source_path);

                let is_in_layered = common_path.exists()
                    || machine_path.exists()
                    || env_path.exists()
                    || self.is_in_layered_subdir(&legacy_path);

                if !is_in_layered {
                    legacy_files.push((entry.source_path.clone(), legacy_path));
                }
            }
        }

        legacy_files
    }

    fn is_in_layered_subdir(&self, path: &Path) -> bool {
        let backup_root = get_backup_root();

        if let Ok(relative) = path.strip_prefix(&backup_root) {
            let first_component = relative
                .components()
                .next()
                .map(|c| c.as_os_str().to_string_lossy().to_string());

            matches!(
                first_component.as_deref(),
                Some("common") | Some("machines") | Some("environments")
            )
        } else {
            false
        }
    }
}

impl Task for MigrateTask {
    fn name(&self) -> &str {
        "Migrate"
    }

    fn execute(&mut self) {
        println!("🔄 Migrating legacy backup files...");
        println!("   Target: {} ({})", self.target, self.profile);

        let target_dir = self.get_target_dir();
        if let Err(e) = fs::create_dir_all(&target_dir) {
            println!("❌ Failed to create target directory: {}", e);
            log(&format!("Failed to create target directory: {}", e));
            return;
        }

        let legacy_files = self.find_legacy_files();

        if legacy_files.is_empty() {
            println!("✅ No legacy files found to migrate.");
            return;
        }

        println!("📋 Found {} legacy files to migrate", legacy_files.len());

        let mut migrated = 0;
        let mut failed = 0;

        for (source_path, legacy_path) in legacy_files {
            let new_path = target_dir.join(&source_path);

            if let Some(parent) = new_path.parent()
                && let Err(e) = fs::create_dir_all(parent)
            {
                println!("⚠️  Failed to create parent dir for {}: {}", source_path, e);
                log(&format!(
                    "Failed to create parent directory for {}: {}",
                    source_path, e
                ));
                failed += 1;
                continue;
            }

            match move_path(&legacy_path, &new_path) {
                Ok(_) => {
                    println!(
                        "✅ Migrated: {} -> {}/{}",
                        source_path, self.target, source_path
                    );
                    log(&format!(
                        "Migrated {} from legacy to {}",
                        source_path, self.target
                    ));
                    migrated += 1;
                }
                Err(e) => {
                    println!("⚠️  Failed to migrate {}: {}", source_path, e);
                    log(&format!("Failed to migrate {}: {}", source_path, e));
                    failed += 1;
                }
            }
        }

        println!(
            "✅ Migration complete. Migrated: {}, Failed: {}",
            migrated, failed
        );
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let target_dir = self.get_target_dir();
        let legacy_files = self.find_legacy_files();

        for (source_path, legacy_path) in legacy_files {
            let new_path = target_dir.join(&source_path);
            operations.push(PlannedOperation::with_target(
                format!("Migrate to {}", self.target),
                format!("{} -> {}", legacy_path.display(), new_path.display()),
            ));
        }

        if operations.is_empty() {
            operations.push(PlannedOperation::with_target(
                "No migration needed".to_string(),
                "All files already in layered structure".to_string(),
            ));
        }

        operations
    }
}

fn move_path(from: &PathBuf, to: &PathBuf) -> std::io::Result<()> {
    if from.is_dir() {
        copy_dir_recursive(from, to)?;
        fs::remove_dir_all(from)?;
    } else {
        fs::copy(from, to)?;
        fs::remove_file(from)?;
    }
    Ok(())
}

fn copy_dir_recursive(from: &PathBuf, to: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(to)?;

    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let dest = to.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest)?;
        } else {
            fs::copy(&path, &dest)?;
        }
    }

    Ok(())
}

pub fn run_with_args(args: crate::cli::MigrateArgs) {
    use crate::tasks::core::TaskExecutor;

    let profile = ActiveProfile::resolve(
        args.profile.as_deref(),
        args.machine_id.as_deref(),
        args.env.as_deref(),
    );

    let target = if args.to_machine {
        MigrateTarget::Machine
    } else if args.to_environment {
        MigrateTarget::Environment
    } else {
        MigrateTarget::Common
    };

    TaskExecutor::run(&mut MigrateTask::new(profile, target), args.dry_run);
}
