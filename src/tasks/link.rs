use crate::logger::log;
use crate::utils::filesystem::{backup_existing_target, copy_dir_to_source};
use crate::utils::paths::{get_base_dirs, get_symlink_backup_path};
use std::fs;
use std::path::Path;

/// Creates symbolic links for dotfiles and configuration directories from `~/dotfiles`
/// to the appropriate system/user paths (e.g., `~/.zshrc`, `~/Library/...`).
pub fn run() {
    println!("🔗 Creating symlinks...");
    log("Starting symlink creation");

    let backup_dir = get_symlink_backup_path();
    if let Err(e) = fs::create_dir_all(&backup_dir) {
        println!("Failed to create backup directory: {e}");
        log(&format!("Failed to create backup directory: {e}"));
        return;
    }

    let base_dirs = get_base_dirs();
    let home_dir = base_dirs.home_dir();
    let dotfiles_dir = home_dir.join("dotfiles");
    let data_dir = base_dirs.data_dir();
    let links = vec![
        (dotfiles_dir.join(".mntn"), home_dir.join(".mntn")),
        (dotfiles_dir.join(".zshrc"), home_dir.join(".zshrc")),
        (dotfiles_dir.join(".vimrc"), home_dir.join(".vimrc")),
        (dotfiles_dir.join("config"), home_dir.join(".config")),
        (
            dotfiles_dir.join("vscode/settings.json"),
            data_dir.join("Code/User/settings.json"),
        ),
        (
            dotfiles_dir.join("vscode/keybindings.json"),
            data_dir.join("Code/User/keybindings.json"),
        ),
    ];

    for (src, dst) in links {
        process_link(&src, &dst, &backup_dir);
    }

    println!("✅ Symlink creation complete.");
    log("Symlink creation complete");
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
fn backup_if_needed(dst: &Path, backup_dir: &Path) -> Result<(), ()> {
    if dst.exists() && !dst.is_symlink() {
        backup_existing_target(dst, backup_dir).map_err(|e| {
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
fn process_link(src: &Path, dst: &Path, backup_dir: &Path) {
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

    if let Err(_) = backup_if_needed(dst, backup_dir) {
        return;
    }

    create_symlink(src, dst);
}
