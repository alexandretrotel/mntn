use std::fs;
use crate::utils::run_cmd;
use crate::logger::log;

pub fn run() {
    let backup_dir = dirs::home_dir().unwrap().join("maintenance/backups");
    fs::create_dir_all(&backup_dir).unwrap();

    println!("🔁 Backing up packages...");
    log("Starting backup");

    let files = vec![
        ("bun.txt", run_cmd("bun", &["pm", "ls", "-g"])),
        ("npm.json", run_cmd("npm", &["ls", "-g", "--depth=0", "--json"])),
        ("uv.txt", run_cmd("uv", &["pip", "freeze"])),
        ("brew.txt", run_cmd("brew", &["leaves"])),
        ("cargo.txt", {
            run_cmd("cargo", &["install", "--list"])
                .lines()
                .filter(|l| !l.is_empty() && !l.contains(' '))
                .collect::<Vec<_>>()
                .join("\n")
        }),
        ("go.txt", run_cmd("go", &["list", "-f", "{{.ImportPath}}", "-m", "all"])),
    ];

    for (name, content) in files {
        fs::write(backup_dir.join(name), content).unwrap();
    }

    println!("✅ Backup complete.");
    log("Backup complete");
}
