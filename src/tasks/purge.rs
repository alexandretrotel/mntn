use crate::logger::log;
use crate::utils::run_cmd;
use inquire::MultiSelect;
use shellexpand::tilde;
use std::fs;

pub fn run() {
    println!("🧼 Listing all launch agents and daemons...");
    log("Starting plist listing");

    let targets = vec![
        ("User LaunchAgents", "~/Library/LaunchAgents"),
        ("System LaunchAgents", "/Library/LaunchAgents"),
        ("System LaunchDaemons", "/Library/LaunchDaemons"),
    ];

    let mut plist_files = Vec::new();

    for (group, raw_path) in &targets {
        let path = tilde(raw_path).to_string();
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                let plist_path = entry.path();
                if plist_path.extension().and_then(|s| s.to_str()) != Some("plist") {
                    continue;
                }

                let label = run_cmd(
                    "defaults",
                    &["read", plist_path.to_str().unwrap_or(""), "Label"],
                )
                .trim()
                .to_string();

                let display_label = if !label.is_empty() {
                    format!("[{}] {}", group, label)
                } else {
                    let fallback = plist_path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("unknown.plist");
                    format!("[{}] {}", group, fallback)
                };

                plist_files.push((display_label, plist_path));
            }
        }
    }

    if plist_files.is_empty() {
        println!("📁 No .plist files found.");
        log("No .plist files found.");
        return;
    }

    let options: Vec<String> = plist_files.iter().map(|(label, _)| label.clone()).collect();

    let to_delete = MultiSelect::new("Select .plist files to delete:", options.clone())
        .prompt()
        .unwrap_or_default();

    for selected in to_delete {
        if let Some((_, path)) = plist_files.iter().find(|(label, _)| label == &selected) {
            let _ = std::process::Command::new("sudo")
                .arg("rm")
                .arg("-f")
                .arg(path)
                .status();
            log(&format!("Deleted: {}", path.display()));
        }
    }

    log("Plist deletion complete");
    println!("✅ Selected files deleted.");
}
