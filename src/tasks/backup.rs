use crate::logger::log;
use crate::tasks::paths::get_backup_path;
use crate::utils::run_cmd;
use std::fs;

pub fn run() {
    let backup_dir = get_backup_path();
    fs::create_dir_all(&backup_dir).unwrap();

    println!("🔁 Backing up packages...");
    log("Starting backup");

    let files: Vec<(&str, Box<dyn Fn() -> String>)> = vec![
        ("bun.txt", Box::new(|| run_cmd("bun", &["pm", "ls", "-g"]))),
        ("npm.txt", Box::new(|| run_cmd("npm", &["ls", "-g"]))),
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

    println!("✅ Backup complete.");
    log("Backup complete");
}
