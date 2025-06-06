use glob::glob;

use crate::logger::log;
use crate::utils::{bytes_to_human_readable, calculate_dir_size, run_cmd};
use shellexpand::tilde;

pub fn run() {
    log("Starting clean");
    println!("🧹 Cleaning system junk...");

    let dirs = vec![
        "~/Library/Caches/*",
        "/Library/Caches/*",
        "/private/var/log/*",
        "~/Library/Logs/*",
        "~/Library/Saved Application State/*",
        "~/Library/Logs/DiagnosticReports/*",
        "/Library/Logs/DiagnosticReports/*",
        "~/.Trash/*",
        "/private/var/root/.Trash/*",
        "/Volumes/*/.Trashes",
    ];

    let mut total_space_saved: u64 = 0;

    for dir in dirs {
        let expanded = tilde(dir).to_string();
        let entries = match glob(&expanded) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.filter_map(Result::ok) {
            if !entry.exists() {
                continue;
            }

            if let Ok(metadata) = entry.symlink_metadata() {
                if metadata.file_type().is_symlink() {
                    continue;
                }
            }

            let space = calculate_dir_size(&entry).unwrap_or(0);
            total_space_saved += space;

            let _ = run_cmd("sudo", &["rm", "-rf", entry.to_str().unwrap()]);
        }
    }

    let _ = run_cmd("qlmanage", &["-r", "cache"]);

    println!("🍺 Running brew cleanup...");
    let _ = run_cmd("brew", &["cleanup"]);

    let space_saved_str = bytes_to_human_readable(total_space_saved);
    println!("✅ System cleaned. Freed {}.", space_saved_str);
    log(&format!("Clean complete. Freed {}.", space_saved_str));
}
