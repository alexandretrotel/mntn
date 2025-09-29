use crate::logger::log;
use crate::registries::configs_registry::ConfigsRegistry;
use crate::utils::filesystem::{backup_existing_target, copy_dir_to_source};
use crate::utils::paths::{get_backup_path, get_base_dirs, get_registry_path, get_symlinks_path};
use std::fs;
use std::path::Path;

/// Creates symbolic links for dotfiles and configuration directories from `~/.mntn/backup`
/// to the appropriate system/user paths (e.g., `~/.zshrc`, `~/Library/...`).
pub fn run() {
    println!("🔗 Creating symlinks...");
    log("Starting symlink creation");

    let symlinks_dir = get_symlinks_path();
    if let Err(e) = fs::create_dir_all(&symlinks_dir) {
        println!("Failed to create symlinks directory: {e}");
        log(&format!("Failed to create symlinks directory: {e}"));
        return;
    }

    // Load the registry
    let registry_path = get_registry_path();
    let registry = match ConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!("❌ Failed to load registry: {}", e);
            log(&format!("Failed to load registry: {}", e));
            return;
        }
    };

    let backup_dir = get_backup_path();
    let base_dirs = get_base_dirs();
    let mut links_processed = 0;

    // Count total enabled entries
    let links_total = registry.get_enabled_entries().count();

    if links_total == 0 {
        println!("ℹ️ No enabled entries found in registry.");
        return;
    }

    println!("📋 Found {} enabled entries in registry", links_total);

    // Process each enabled entry from the registry
    for (id, entry) in registry.get_enabled_entries() {
        let src = backup_dir.join(&entry.source_path);

        let dst = match entry.target_path.resolve(&base_dirs) {
            Ok(path) => path,
            Err(e) => {
                println!("⚠️ Failed to resolve path for {}: {}", entry.name, e);
                log(&format!("Failed to resolve path for {}: {}", entry.name, e));
                continue;
            }
        };

        println!("🔗 Processing: {} ({})", entry.name, id);
        process_link(&src, &dst, &symlinks_dir);
        links_processed += 1;
    }

    println!(
        "✅ Symlink creation complete. Processed {}/{} entries.",
        links_processed, links_total
    );
    log(&format!(
        "Symlink creation complete. Processed {}/{} entries",
        links_processed, links_total
    ));
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
        Ok(_) => log(&format!("Linked {} → {}", src.display(), dst.display())),
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
    if let Err(_) = copy_dst_to_src_if_missing(src, dst) {
        return;
    }

    if !src.exists() {
        log(&format!(
            "Warning: Source {} does not exist. Skipping...",
            src.display()
        ));
        return;
    }

    if let Err(_) = handle_existing_symlink(src, dst) {
        return;
    }

    if let Err(_) = backup_if_needed(dst, symlinks_dir) {
        return;
    }

    create_symlink(src, dst);
}
