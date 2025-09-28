use crate::logger::log;
use crate::utils::app_paths::{
    get_ghostty_config_path, get_vscode_keybindings_path, get_vscode_settings_path,
};
use crate::utils::paths::get_backup_path;
use crate::utils::system::run_cmd;
use std::fs;
use std::path::PathBuf;

/// Attempts to back up a given editor or config file by reading its contents and writing to the backup directory.
///
/// If the file path is `None`, it logs and prints a warning message.
fn backup_editor_file(
    path: Option<PathBuf>,
    backup_name: &str,
    backup_dir: &PathBuf,
) -> std::io::Result<()> {
    let file_path = match path {
        Some(p) => p,
        None => {
            log(&format!("{} not found", backup_name));
            eprintln!("⚠️ {} not found.", backup_name);
            return Ok(());
        }
    };

    let contents = fs::read_to_string(&file_path)?;
    let backup_path = backup_dir.join(backup_name);

    fs::write(&backup_path, contents)?;
    log(&format!("Backed up {}", backup_name));
    println!("🔁 Backed up {}", backup_name);

    Ok(())
}

/// Runs the full backup process.
///
/// This function:
/// - Ensures the backup directory exists.
/// - Logs and prints start and completion messages.
/// - Collects global package lists from various package managers, saving each to individual text files.
/// - Backs up key editor configuration files for VSCode and Ghostty.
pub fn run() {
    let backup_dir = get_backup_path();
    fs::create_dir_all(&backup_dir).unwrap();

    println!("🔁 Backing up packages...");
    log("Starting backup");

    // List of package managers and their backup files + commands
    let files: Vec<(&str, Box<dyn Fn() -> String>)> = vec![
        ("bun.txt", Box::new(|| run_cmd("bun", &["pm", "ls", "-g"]))),
        ("npm.txt", Box::new(|| run_cmd("npm", &["ls", "-g"]))),
        ("pnpm.txt", Box::new(|| run_cmd("pnpm", &["ls", "-g"]))),
        (
            "yarn.txt",
            Box::new(|| run_cmd("yarn", &["global", "list"])),
        ),
        (
            "pip.txt",
            Box::new(|| run_cmd("pip", &["list", "--format=freeze"])),
        ),
        ("uv.txt", Box::new(|| run_cmd("uv", &["pip", "freeze"]))),
        ("brew.txt", Box::new(|| run_cmd("brew", &["leaves"]))),
        (
            "brew-cask.txt",
            Box::new(|| run_cmd("brew", &["list", "--cask"])),
        ),
        (
            "cargo.txt",
            Box::new(|| run_cmd("cargo", &["install", "--list"])),
        ),
        (
            "go.txt",
            Box::new(|| run_cmd("go", &["list", "-f", "{{.ImportPath}}", "-m", "all"])),
        ),
    ];

    // Execute each command and write output to corresponding file
    for (name, cmd_fn) in files {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(cmd_fn));

        match result {
            Ok(content) => {
                if let Err(e) = fs::write(backup_dir.join(&name), content) {
                    eprintln!("Failed to write {}: {}", name, e);
                    log(&format!("Failed to write {}: {}", name, e));
                }
            }
            Err(_) => {
                eprintln!("Command for {} panicked", name);
                log(&format!("Command for {} panicked", name));
                let _ = fs::write(backup_dir.join(&name), "");
            }
        }
    }

    // Backup editor/config files
    let editor_files = vec![
        (get_vscode_settings_path(), "vscode-settings.json"),
        (get_vscode_keybindings_path(), "vscode-keybindings.json"),
        (get_ghostty_config_path(), "ghostty-config"),
    ];

    for (path, backup_name) in editor_files {
        let _ = backup_editor_file(path, backup_name, &backup_dir);
    }

    println!("✅ Backup complete.");
    log("Backup complete");
}
