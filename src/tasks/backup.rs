use crate::logger::log;
use crate::tasks::paths::get_backup_path;
use crate::utils::{
    get_cursor_keybindings_path, get_cursor_settings_path, get_ghostty_config_path,
    get_iterm_preferences_path, get_vscode_keybindings_path, get_vscode_settings_path, run_cmd,
};
use std::fs;
use std::path::PathBuf;

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

fn backup_binary_file(path: Option<PathBuf>, backup_name: &str, backup_dir: &PathBuf) {
    if let Some(file_path) = path {
        match fs::read(&file_path) {
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

pub fn run() {
    let backup_dir = get_backup_path();
    fs::create_dir_all(&backup_dir).unwrap();

    println!("🔁 Backing up packages...");
    log("Starting backup");

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
        ("pipx.txt", Box::new(|| run_cmd("pipx", &["list"]))),
        ("gem.txt", Box::new(|| run_cmd("gem", &["list"]))),
        (
            "composer.txt",
            Box::new(|| run_cmd("composer", &["global", "show"])),
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
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (cmd_fn)()));
        let content = result.unwrap_or_else(|_| String::new());
        let _ = fs::write(backup_dir.join(name), content);
    }

    // Backup editor configuration files
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
    backup_editor_file(
        get_cursor_settings_path(),
        "cursor-settings.json",
        &backup_dir,
    );
    backup_editor_file(
        get_cursor_keybindings_path(),
        "cursor-keybindings.json",
        &backup_dir,
    );
    backup_binary_file(
        get_iterm_preferences_path(),
        "iterm-preferences.plist",
        &backup_dir,
    );
    backup_editor_file(get_ghostty_config_path(), "ghostty-config", &backup_dir);

    println!("✅ Backup complete.");
    log("Backup complete");
}
