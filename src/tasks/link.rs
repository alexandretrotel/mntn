use crate::logger::log;
use crate::utils::filesystem::{backup_existing_target, copy_dir_to_source};
use crate::utils::paths::{get_base_dirs, get_symlink_backup_path};
use std::fs;

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
        (dotfiles_dir.join(".zshrc"), home_dir.join(".zshrc")),
        (dotfiles_dir.join(".vimrc"), home_dir.join(".vimrc")),
        (dotfiles_dir.join("config"), home_dir.join(".config")),
        (
            dotfiles_dir.join("config/vscode/settings.json"),
            data_dir.join("Code/User/settings.json"),
        ),
        (
            dotfiles_dir.join("config/vscode/keybindings.json"),
            data_dir.join("Code/User/keybindings.json"),
        ),
    ];

    for (src, dst) in links {
        if dst.exists() && !dst.is_symlink() && !src.exists() {
            if dst.is_file() {
                if let Err(e) = fs::copy(&dst, &src) {
                    log(&format!(
                        "Failed to copy file {} to source {}: {}",
                        dst.display(),
                        src.display(),
                        e
                    ));
                    continue;
                }
            } else if dst.is_dir() {
                if let Err(e) = copy_dir_to_source(&dst, &src) {
                    log(&format!(
                        "Failed to copy directory {} to source {}: {}",
                        dst.display(),
                        src.display(),
                        e
                    ));
                    continue;
                }
            } else {
                log(&format!(
                    "Unknown target type for {}. Skipping.",
                    dst.display()
                ));
                continue;
            }
        }

        if !src.exists() {
            log(&format!(
                "Warning: Source {} does not exist. Skipping...",
                src.display()
            ));
            continue;
        }

        if dst.is_symlink() {
            match fs::read_link(&dst) {
                Ok(existing) if existing == src => {
                    log(&format!(
                        "Symlink {} already correctly points to {}",
                        dst.display(),
                        src.display()
                    ));
                    continue;
                }
                Ok(existing) => {
                    log(&format!(
                        "Removing incorrect symlink {} → {}",
                        dst.display(),
                        existing.display()
                    ));
                    if let Err(e) = fs::remove_file(&dst) {
                        log(&format!(
                            "Failed to remove incorrect symlink {}: {}",
                            dst.display(),
                            e
                        ));
                        continue;
                    }
                }
                Err(e) => {
                    log(&format!("Failed to read symlink {}: {}", dst.display(), e));
                    continue;
                }
            }
        }

        if dst.exists() && !dst.is_symlink() {
            if let Err(e) = backup_existing_target(&dst, &backup_dir) {
                log(&format!("Failed to back up {}: {}", dst.display(), e));
                continue;
            }
        }

        if let Err(e) = std::os::unix::fs::symlink(&src, &dst) {
            log(&format!(
                "Failed to link {} → {}: {}",
                src.display(),
                dst.display(),
                e
            ));
        } else {
            log(&format!("Linked {} → {}", src.display(), dst.display()));
        }
    }

    println!("✅ Symlink creation complete.");
    log("Symlink creation complete");
}
