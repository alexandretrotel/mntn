use crate::logger::log;
use crate::tasks::paths::get_symlink_backup_path;
use shellexpand::tilde;
use std::fs;
use std::path::Path;
use std::process;

pub fn run() {
    println!("🔗 Creating symlinks...");
    log("Starting symlink creation");

    // Define backup directory for symlinks
    let backup_dir = get_symlink_backup_path();
    fs::create_dir_all(&backup_dir).unwrap(); // Create ~/dotfiles/backups/symlinks

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

        // Check if source exists
        if !Path::new(&source_path).exists() {
            println!("Error: {} does not exist", source_path);
            log(&format!("Error: Source {} does not exist", source_path));
            process::exit(1);
        }

        // Create symlink with backup
        let target_path = Path::new(&target_path);
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
            println!(
                "Backing up {} to {}",
                target_path.display(),
                backup_path.display()
            );
            log(&format!(
                "Backing up {} to {}",
                target_path.display(),
                backup_path.display()
            ));
            if let Err(e) = fs::rename(target_path, &backup_path) {
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
            println!("Failed to link {}: {}", target_path.display(), e);
            log(&format!("Failed to link {}: {}", target_path.display(), e));
        } else {
            println!("Linked {} to {}", source_path, target_path.display());
            log(&format!(
                "Linked {} to {}",
                source_path,
                target_path.display()
            ));
        }
    }

    println!("✅ Symlink creation complete.");
    log("Symlink creation complete");
}
