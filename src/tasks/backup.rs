use crate::logger::log;
use crate::tasks::paths::get_backup_path;
use crate::utils::{
    get_ghostty_config_path, get_vscode_keybindings_path, get_vscode_settings_path, run_cmd,
};
use std::fs;
use std::path::PathBuf;

/// Attempts to back up a given editor or config file by reading its contents and writing to the backup directory.
///
/// If the file path is `None`, logs and prints a warning indicating the file was not found.
///
/// # Arguments
///
/// * `path` - Optional path to the file to back up.
/// * `backup_name` - The name to save the backed-up file as in the backup directory.
/// * `backup_dir` - Directory path where backup files are stored.
///
/// # Behavior
///
/// Prints and logs success or failure messages.
/// Fails gracefully if reading or writing the file fails.
fn backup_editor_file(path: Option<PathBuf>, backup_name: &str, backup_dir: &PathBuf) {
    if let Some(file_path) = path {
        match fs::read_to_string(&file_path) {
            Ok(contents) => {
                let _ = fs::write(backup_dir.join(backup_name), contents);
                println!("🔁 Backed up {}", backup_name);
                log(&format!("Backed up {}", backup_name));
            }
            Err(e) => {
                println!("⚠️ Failed to read {}: {}", backup_name, e);
                log(&format!("Failed to read {}: {}", backup_name, e));
            }
        }
    } else {
        println!("⚠️ {} not found.", backup_name);
        log(&format!("{} not found", backup_name));
    }
}

/// Runs the full backup process.
///
/// This function:
/// - Ensures the backup directory exists.
/// - Logs and prints start and completion messages.
/// - Collects global package lists from various package managers, saving each to individual text files.
/// - Backs up key editor configuration files for VSCode and Ghostty.
///
/// # Panics
///
/// Panics if the backup directory cannot be created.
///
/// # Details
///
/// Commands executed for packages include bun, npm, pnpm, yarn, pip, uv, brew (and brew cask), cargo, and go.
/// Any panics from these commands are caught and replaced with empty output, allowing backup to continue.
///
/// # Side Effects
///
/// Prints progress and error messages to stdout and logs them via `log`.
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

    for (name, cmd_fn) in files {
        // Catch panics in command execution so backup continues even if one fails
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (cmd_fn)()));
        let content = result.unwrap_or_else(|_| String::new());
        let _ = fs::write(backup_dir.join(name), content);
    }

    // Backup editor/config files
    backup_editor_file(
        get_vscode_settings_path(),
        "vscode-settings.json",
        &backup_dir,
    );
    backup_editor_file(
        get_vscode_keybindings_path(),
        "vscode-keybindings.json",
        &backup_dir,
    );
    backup_editor_file(get_ghostty_config_path(), "ghostty-config", &backup_dir);

    println!("✅ Backup complete.");
    log("Backup complete");
}
