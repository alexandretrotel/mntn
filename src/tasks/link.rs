use crate::logger::log;
use crate::tasks::paths::get_symlink_backup_path;
use shellexpand::tilde;
use std::fs::{self};
use std::path::Path;

pub fn run() {
    println!("🔗 Creating symlinks...");
    log("Starting symlink creation");

    let backup_dir = get_symlink_backup_path();
    fs::create_dir_all(&backup_dir).expect("Failed to create backup directory");

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

    for (source, target) in links {
        let source_path = tilde(source).to_string();
        let target_path = tilde(target).to_string();
        let source_path = Path::new(&source_path);
        let target_path = Path::new(&target_path);

        if target_path.exists() && !target_path.is_symlink() && !source_path.exists() {
            if target_path.is_file() {
                log(&format!(
                    "Copying {} to {}",
                    target_path.display(),
                    source_path.display()
                ));

                if let Some(parent) = source_path.parent() {
                    fs::create_dir_all(parent)
                        .expect("Failed to create parent directories for source");
                }

                if let Err(e) = fs::copy(&target_path, &source_path) {
                    println!(
                        "Failed to copy file {} to {}: {}",
                        target_path.display(),
                        source_path.display(),
                        e
                    );
                    log(&format!(
                        "Failed to copy file {} to {}: {}",
                        target_path.display(),
                        source_path.display(),
                        e
                    ));
                    continue;
                }
            } else if target_path.is_dir() {
                log(&format!(
                    "Copying directory {} to {}",
                    target_path.display(),
                    source_path.display()
                ));

                if let Some(parent) = source_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        println!(
                            "Failed to create parent directories for {}: {}",
                            source_path.display(),
                            e
                        );
                        log(&format!(
                            "Failed to create parent directories for {}: {}",
                            source_path.display(),
                            e
                        ));
                        continue;
                    }
                }

                let mut options = fs_extra::dir::CopyOptions::new();
                options.copy_inside = true; // copy contents, not root dir itself
                if let Err(e) = fs_extra::dir::copy(&target_path, &source_path, &options) {
                    println!(
                        "Failed to copy directory {} to {}: {}",
                        target_path.display(),
                        source_path.display(),
                        e
                    );
                    log(&format!(
                        "Failed to copy directory {} to {}: {}",
                        target_path.display(),
                        source_path.display(),
                        e
                    ));
                    continue;
                }

                continue;
            } else {
                println!(
                    "Unknown target type for {}. Skipping.",
                    target_path.display()
                );
                log(&format!(
                    "Unknown target type for {}. Skipping.",
                    target_path.display()
                ));
                continue;
            }
        }

        if !source_path.exists() {
            log(&format!(
                "Warning: Source {} does not exist. Skipping...",
                source_path.display()
            ));
            continue;
        }

        if target_path.is_symlink() {
            match fs::read_link(&target_path) {
                Ok(existing_link) => {
                    if existing_link == source_path {
                        log(&format!(
                            "Symlink {} already correctly points to {}",
                            target_path.display(),
                            source_path.display()
                        ));
                        continue; // Skip to next pair
                    } else {
                        log(&format!(
                            "Removing incorrect symlink {} pointing to {}",
                            target_path.display(),
                            existing_link.display()
                        ));
                        if let Err(e) = fs::remove_file(&target_path) {
                            println!(
                                "Failed to remove incorrect symlink {}: {}",
                                target_path.display(),
                                e
                            );
                            log(&format!(
                                "Failed to remove incorrect symlink {}: {}",
                                target_path.display(),
                                e
                            ));
                            continue;
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to read symlink {}: {}", target_path.display(), e);
                    log(&format!(
                        "Failed to read symlink {}: {}",
                        target_path.display(),
                        e
                    ));
                    continue;
                }
            }
        }

        if target_path.exists() && !target_path.is_symlink() {
            let target_file_name = target_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "backup".to_string());
            let backup_path = backup_dir.join(format!(
                "{}_{}",
                target_file_name,
                chrono::Local::now().format("%Y%m%d_%H%M%S")
            ));
            log(&format!(
                "Backing up {} to {}",
                target_path.display(),
                backup_path.display()
            ));
            if let Err(e) = fs::rename(&target_path, &backup_path) {
                println!("Failed to back up {}: {}", target_path.display(), e);
                log(&format!(
                    "Failed to back up {}: {}",
                    target_path.display(),
                    e
                ));
                continue;
            }
        }

        if let Err(e) = std::os::unix::fs::symlink(&source_path, &target_path) {
            println!(
                "Failed to link {} to {}: {}",
                source_path.display(),
                target_path.display(),
                e
            );
            log(&format!(
                "Failed to link {} to {}: {}",
                source_path.display(),
                target_path.display(),
                e
            ));
        } else {
            log(&format!(
                "Linked {} to {}",
                source_path.display(),
                target_path.display()
            ));
        }
    }

    println!("✅ Symlink creation complete.");
    log("Symlink creation complete");
}
