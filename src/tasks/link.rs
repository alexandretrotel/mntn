use crate::logger::log;
use crate::tasks::paths::get_symlink_backup_path;
use shellexpand::tilde;
use std::fs::{self};
use std::path::Path;

pub fn run() {
    println!("🔗 Creating symlinks...");
    log("Starting symlink creation");

    // Define backup directory for existing files
    let backup_dir = get_symlink_backup_path();
    fs::create_dir_all(&backup_dir).expect("Failed to create backup directory");

    // Define source-target pairs
    let links = vec![
        ("~/dotfiles/.zshrc", "~/.zshrc"),
        ("~/dotfiles/.vimrc", "~/.vimrc"),
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

        // Special case for VSCode settings.json
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
                println!(
                    "Skipping copy of directory {}: Source does not exist and copying directories is not supported",
                    target_path.display()
                );
                log(&format!(
                    "Skipping copy of directory {}: Source does not exist and copying directories is not supported",
                    target_path.display()
                ));
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

        // Check if source exists
        if !source_path.exists() {
            log(&format!(
                "Warning: Source {} does not exist. Skipping...",
                source_path.display()
            ));
            continue;
        }

        // Check if target is already a correct symlink
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

        // Backup existing file if it’s not a symlink
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

        // Create or update symlink
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
