use glob::glob;

use crate::logger::log;
use crate::utils::filesystem::calculate_dir_size;
use crate::utils::format::bytes_to_human_readable;
use crate::utils::system::run_cmd;
use shellexpand::tilde;

/// Performs a system junk cleanup by deleting cache, logs, trash, and other temporary files on macOS.
///
/// The cleanup targets multiple common system and user directories, including:
/// - User and system caches (`~/Library/Caches`, `/Library/Caches`)
/// - System and user logs (`/private/var/log`, `~/Library/Logs`, etc.)
/// - Saved application states and diagnostic reports
/// - Trash directories for both user and root
/// - Volume trashes (`/Volumes/*/.Trashes`)
///
/// For each target directory, the function:
/// - Expands the tilde to the user's home directory.
/// - Uses globbing to find matching entries.
/// - Skips symlinks to avoid unintended deletions.
/// - Calculates total size of deleted items.
/// - Removes the files/directories using `sudo rm -rf`.
///
/// Additionally:
/// - Resets Quick Look cache by running `qlmanage -r cache`.
/// - Runs `brew cleanup` to clean Homebrew caches.
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
            Err(_) => continue, // skip invalid patterns
        };

        for entry in entries.filter_map(Result::ok) {
            if !entry.exists() {
                continue;
            }

            if let Ok(metadata) = entry.symlink_metadata() {
                if metadata.file_type().is_symlink() {
                    continue; // skip symlinks
                }
            }

            let space = calculate_dir_size(&entry).unwrap_or(0);
            total_space_saved += space;

            // Remove the file/directory forcefully with sudo
            let _ = run_cmd("sudo", &["rm", "-rf", entry.to_str().unwrap()]);
        }
    }

    // Reset Quick Look cache
    let _ = run_cmd("qlmanage", &["-r", "cache"]);

    println!("🍺 Running brew cleanup...");
    let _ = run_cmd("brew", &["cleanup"]);

    let space_saved_str = bytes_to_human_readable(total_space_saved);
    println!("✅ System cleaned. Freed {}.", space_saved_str);
    log(&format!("Clean complete. Freed {}.", space_saved_str));
}
