use crate::logger::log;
use crate::tasks::paths::get_symlink_backup_path;
use crate::utils::filesystem::{backup_existing_target, copy_dir_to_source, copy_file_to_source};
use shellexpand::tilde;
use std::fs;
use std::path::PathBuf;

/// Creates symbolic links for dotfiles and configuration directories from `~/dotfiles`
/// to the appropriate system/user paths (e.g., `~/.zshrc`, `~/Library/...`).
///
/// ## Behavior
/// - Ensures a backup directory exists (via `get_symlink_backup_path()`).
/// - For each pair of source and target:
///     - If the source does not exist but the target does, copies the target to the source.
///     - If the target exists and is not a correct symlink, backs it up.
///     - If the target is a symlink pointing elsewhere, removes it.
///     - Creates a symlink from the source to the target path.
/// - All failures are logged, and the function proceeds with remaining links.
///
/// ## Notes
/// - Logs are written via the `log()` utility.
/// - Errors do not panic the program but are gracefully logged and skipped.
pub fn run() {
    println!("🔗 Creating symlinks...");
    log("Starting symlink creation");

    let backup_dir = get_symlink_backup_path();
    if let Err(e) = fs::create_dir_all(&backup_dir) {
        println!("Failed to create backup directory: {e}");
        log(&format!("Failed to create backup directory: {e}"));
        return;
    }

    let links = vec![
        ("~/dotfiles/.zshrc", "~/.zshrc"),
        ("~/dotfiles/.vimrc", "~/.vimrc"),
        ("~/dotfiles/vim_runtime", "~/.vim_runtime"),
        ("~/dotfiles/config", "~/.config"),
        (
            "~/dotfiles/config/lporg",
            "~/Library/Application Support/lporg",
        ),
        (
            "~/dotfiles/config/vscode/settings.json",
            "~/Library/Application Support/Code/User/settings.json",
        ),
    ];

    for (src, dst) in links {
        let source = PathBuf::from(tilde(src).to_string());
        let target = PathBuf::from(tilde(dst).to_string());

        if target.exists() && !target.is_symlink() && !source.exists() {
            if target.is_file() {
                if let Err(e) = copy_file_to_source(&target, &source) {
                    log(&format!(
                        "Failed to copy file {} to source {}: {}",
                        target.display(),
                        source.display(),
                        e
                    ));
                    continue;
                }
            } else if target.is_dir() {
                if let Err(e) = copy_dir_to_source(&target, &source) {
                    log(&format!(
                        "Failed to copy directory {} to source {}: {}",
                        target.display(),
                        source.display(),
                        e
                    ));
                    continue;
                }
            } else {
                log(&format!(
                    "Unknown target type for {}. Skipping.",
                    target.display()
                ));
                continue;
            }
        }

        if !source.exists() {
            log(&format!(
                "Warning: Source {} does not exist. Skipping...",
                source.display()
            ));
            continue;
        }

        if target.is_symlink() {
            match fs::read_link(&target) {
                Ok(existing) if existing == source => {
                    log(&format!(
                        "Symlink {} already correctly points to {}",
                        target.display(),
                        source.display()
                    ));
                    continue;
                }
                Ok(existing) => {
                    log(&format!(
                        "Removing incorrect symlink {} → {}",
                        target.display(),
                        existing.display()
                    ));
                    if let Err(e) = fs::remove_file(&target) {
                        log(&format!(
                            "Failed to remove incorrect symlink {}: {}",
                            target.display(),
                            e
                        ));
                        continue;
                    }
                }
                Err(e) => {
                    log(&format!(
                        "Failed to read symlink {}: {}",
                        target.display(),
                        e
                    ));
                    continue;
                }
            }
        }

        if target.exists() && !target.is_symlink() {
            if let Err(e) = backup_existing_target(&target, &backup_dir) {
                log(&format!("Failed to back up {}: {}", target.display(), e));
                continue;
            }
        }

        if let Err(e) = std::os::unix::fs::symlink(&source, &target) {
            log(&format!(
                "Failed to link {} → {}: {}",
                source.display(),
                target.display(),
                e
            ));
        } else {
            log(&format!(
                "Linked {} → {}",
                source.display(),
                target.display()
            ));
        }
    }

    println!("✅ Symlink creation complete.");
    log("Symlink creation complete");
}
