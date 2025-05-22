use crate::logger::log;
use crate::utils::run_cmd;
use shellexpand::tilde;
use std::fs;
use std::io::{self, Write};

pub fn run() {
    println!("🧼 Purging unused launch agents and daemons...");
    log("Starting purge");

    let paths = vec![
        "~/Library/LaunchAgents",
        "/Library/LaunchAgents",
        "/Library/LaunchDaemons",
    ];

    for raw_path in paths {
        let path = tilde(raw_path).to_string();
        let entries = fs::read_dir(&path);
        if entries.is_err() {
            continue;
        }

        for entry in entries.unwrap().flatten() {
            let plist_path = entry.path();
            if plist_path.extension().and_then(|s| s.to_str()) != Some("plist") {
                continue;
            }

            let label_output =
                run_cmd("defaults", &["read", plist_path.to_str().unwrap(), "Label"]);
            let label = label_output.trim();

            let loaded = run_cmd("launchctl", &["list"]);
            if label.is_empty() || !loaded.contains(label) {
                println!("Unused: {}", plist_path.display());
                print!("Delete this file? [y/N]: ");
                io::stdout().flush().unwrap();

                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm).unwrap();

                if confirm.trim().eq_ignore_ascii_case("y") {
                    let _ = std::process::Command::new("sudo")
                        .arg("rm")
                        .arg("-f")
                        .arg(plist_path.to_str().unwrap())
                        .status();
                    log(&format!("Purged: {}", plist_path.display()));
                } else {
                    log(&format!("Skipped: {}", plist_path.display()));
                }
            }
        }
    }

    log("Purge complete");
    println!("✅ Purge complete.");
}
