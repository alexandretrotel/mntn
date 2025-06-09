use crate::logger::log;
use crate::tasks::paths::get_backup_path;
use crate::utils::run_cmd;
use std::fs;
use std::path::PathBuf;

fn get_vscode_settings_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let vscode_path = home_dir.join("Library/Application Support/Code/User/settings.json");
    if vscode_path.exists() {
        Some(vscode_path)
    } else {
        None
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

    if let Some(vscode_settings_path) = get_vscode_settings_path() {
        match fs::read_to_string(&vscode_settings_path) {
            Ok(contents) => {
                let _ = fs::write(backup_dir.join("vscode-settings.json"), contents);
                println!("🔁 Backed up VSCode settings.json");
                log("Backed up VSCode settings.json");
            }
            Err(e) => {
                println!("⚠️ Failed to read VSCode settings.json: {}", e);
                log("Failed to read VSCode settings.json");
            }
        }
    } else {
        println!("⚠️ VSCode settings.json not found.");
        log("VSCode settings.json not found");
    }

    println!("✅ Backup complete.");
    log("Backup complete");
}
