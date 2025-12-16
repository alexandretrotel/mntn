use crate::logger::log;
use crate::profile::ActiveProfile;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::tasks::core::{PlannedOperation, Task};
use crate::utils::filesystem::{backup_existing_target, copy_dir_to_source};
use crate::utils::paths::{get_registry_path, get_symlinks_path};
use std::fs;
use std::path::Path;

pub struct LinkTask {
    profile: ActiveProfile,
}

impl LinkTask {
    pub fn new(profile: ActiveProfile) -> Self {
        Self { profile }
    }
}

impl Task for LinkTask {
    fn name(&self) -> &str {
        "Link"
    }

    fn execute(&mut self) {
        println!("🔗 Creating symlinks...");
        println!("   Profile: {}", self.profile);

        let symlinks_dir = get_symlinks_path();
        if let Err(e) = fs::create_dir_all(&symlinks_dir) {
            println!("Failed to create symlinks directory: {e}");
            log(&format!("Failed to create symlinks directory: {e}"));
            return;
        }

        let registry_path = get_registry_path();
        let registry = match ConfigsRegistry::load_or_create(&registry_path) {
            Ok(registry) => registry,
            Err(e) => {
                println!("❌ Failed to load registry: {}", e);
                log(&format!("Failed to load registry: {}", e));
                return;
            }
        };

        let mut links_processed = 0;
        let mut links_skipped = 0;
        let links_total = registry.get_enabled_entries().count();

        if links_total == 0 {
            println!("ℹ️ No enabled entries found in registry.");
            return;
        }

        println!("📋 Found {} enabled entries in registry", links_total);

        for (id, entry) in registry.get_enabled_entries() {
            let dst = &entry.target_path;

            match self.profile.resolve_source(&entry.source_path) {
                Some(resolved) => {
                    println!(
                        "🔗 Processing: {} ({}) [{}]",
                        entry.name, id, resolved.layer
                    );
                    process_link(&resolved.path, dst, &symlinks_dir);
                    links_processed += 1;
                }
                None => {
                    println!(
                        "⚠️  Skipping: {} ({}) - no source found in any layer",
                        entry.name, id
                    );
                    log(&format!(
                        "No source found for {} in any layer (checked: environment/{}, machines/{}, common, legacy)",
                        entry.source_path, self.profile.environment, self.profile.machine_id
                    ));
                    links_skipped += 1;
                }
            }
        }

        println!(
            "✅ Symlink creation complete. Processed: {}, Skipped: {}",
            links_processed, links_skipped
        );
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();

        if let Ok(registry) = ConfigsRegistry::load_or_create(&get_registry_path()) {
            for (_id, entry) in registry.get_enabled_entries() {
                let dst = &entry.target_path;

                match self.profile.resolve_source(&entry.source_path) {
                    Some(resolved) => {
                        operations.push(PlannedOperation::with_target(
                            format!("Link {} [{}]", entry.name, resolved.layer),
                            format!("{} -> {}", dst.display(), resolved.path.display()),
                        ));
                    }
                    None => {
                        operations.push(PlannedOperation::with_target(
                            format!("Skip {} (no source)", entry.name),
                            format!("{} -> ???", dst.display()),
                        ));
                    }
                }
            }
        }

        operations
    }
}

pub fn run_with_args(args: crate::cli::LinkArgs) {
    use crate::tasks::core::TaskExecutor;

    let profile = args.profile_args.resolve();

    TaskExecutor::run(&mut LinkTask::new(profile), args.dry_run);
}

/// Copies from dst to src if src is missing, handling both files and directories
fn copy_dst_to_src_if_missing(src: &Path, dst: &Path) -> Result<(), ()> {
    if dst.exists() && !dst.is_symlink() && !src.exists() {
        if dst.is_file() {
            fs::copy(dst, src).map_err(|e| {
                log(&format!(
                    "Failed to copy file {} to source {}: {}",
                    dst.display(),
                    src.display(),
                    e
                ));
            })?;
        } else if dst.is_dir() {
            copy_dir_to_source(dst, src).map_err(|e| {
                log(&format!(
                    "Failed to copy directory {} to source {}: {}",
                    dst.display(),
                    src.display(),
                    e
                ));
            })?;
        } else {
            log(&format!(
                "Unknown target type for {}. Skipping.",
                dst.display()
            ));
            return Err(());
        }
    }
    Ok(())
}

/// Handles existing symlink logic: checks if it's correct, removes if wrong
fn handle_existing_symlink(src: &Path, dst: &Path) -> Result<(), ()> {
    if dst.is_symlink() {
        match fs::read_link(dst) {
            Ok(existing) if existing == src => {
                log(&format!(
                    "Symlink {} already correctly points to {}",
                    dst.display(),
                    src.display()
                ));
                return Err(()); // nothing more to do
            }
            Ok(existing) => {
                log(&format!(
                    "Removing incorrect symlink {} → {}",
                    dst.display(),
                    existing.display()
                ));
                fs::remove_file(dst).map_err(|e| {
                    log(&format!(
                        "Failed to remove incorrect symlink {}: {}",
                        dst.display(),
                        e
                    ));
                })?;
            }
            Err(e) => {
                log(&format!("Failed to read symlink {}: {}", dst.display(), e));
                return Err(());
            }
        }
    }
    Ok(())
}

/// Backs up the destination if it exists and is not a symlink
fn backup_if_needed(dst: &Path, symlinks_dir: &Path) -> Result<(), ()> {
    if dst.exists() && !dst.is_symlink() {
        backup_existing_target(dst, symlinks_dir).map_err(|e| {
            log(&format!("Failed to back up {}: {}", dst.display(), e));
        })?;
    }
    Ok(())
}

/// Creates a symlink from src to dst
fn create_symlink(src: &Path, dst: &Path) {
    match std::os::unix::fs::symlink(src, dst) {
        Ok(()) => log(&format!("Linked {} → {}", src.display(), dst.display())),
        Err(e) => log(&format!(
            "Failed to link {} → {}: {}",
            src.display(),
            dst.display(),
            e
        )),
    }
}

/// Processes a single (src, dst) link
fn process_link(src: &Path, dst: &Path, symlinks_dir: &Path) {
    if copy_dst_to_src_if_missing(src, dst).is_err() {
        return;
    }

    if !src.exists() {
        log(&format!(
            "Warning: Source {} does not exist. Skipping...",
            src.display()
        ));
        return;
    }

    if handle_existing_symlink(src, dst).is_err() {
        return;
    }

    if backup_if_needed(dst, symlinks_dir).is_err() {
        return;
    }

    create_symlink(src, dst);
}
