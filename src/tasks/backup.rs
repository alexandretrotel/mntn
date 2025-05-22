use crate::logger::log;
use crate::tasks::paths::get_backup_path;
use crate::utils::run_cmd;
use std::fs;

pub fn run() {
    let backup_dir = get_backup_path();
    fs::create_dir_all(&backup_dir).unwrap();

    println!("🔁 Backing up packages...");
    log("Starting backup");

    let files = vec![
        ("bun.txt", run_cmd("bun", &["pm", "ls", "-g"])),
        ("npm.txt", run_cmd("npm", &["ls", "-g"])),
        ("uv.txt", run_cmd("uv", &["pip", "freeze"])),
        ("brew.txt", run_cmd("brew", &["leaves"])),
        ("cargo.txt", { run_cmd("cargo", &["install", "--list"]) }),
        (
            "go.txt",
            run_cmd("go", &["list", "-f", "{{.ImportPath}}", "-m", "all"]),
        ),
    ];

    for (name, content) in files {
        fs::write(backup_dir.join(name), content).unwrap();
    }

    println!("✅ Backup complete.");
    log("Backup complete");
}
