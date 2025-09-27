use crate::logger::log;
use crate::utils::system::run_cmd;
use inquire::MultiSelect;
use shellexpand::tilde;
use std::fs;
use std::path::PathBuf;

/// Lists all macOS LaunchAgents and LaunchDaemons `.plist` files from standard system directories,
/// prompts the user to select which ones to delete, and deletes the selected files using `sudo rm`.
///
/// # Behavior
/// - Scans three directories:
///   - User LaunchAgents: `~/Library/LaunchAgents`
///   - System LaunchAgents: `/Library/LaunchAgents`
///   - System LaunchDaemons: `/Library/LaunchDaemons`
/// - For each `.plist` file found, attempts to read the Label via the `defaults` command to display a friendly name.
/// - Presents an interactive multi-selection prompt to delete chosen `.plist` files.
/// - Deletes selected files with `sudo rm -f`.
/// - Logs each deletion and the overall process status.
///
/// # Notes
/// - Uses `shellexpand` to expand tilde in user path.
/// - Ignores `.plist` files without a readable Label, displaying filename instead.
/// - Handles failures silently for file deletion (does not report deletion errors to user).
/// - If no `.plist` files are found, logs and prints a message, then exits.
///
/// # Example
/// ```no_run
/// plist::run();
/// ```
pub fn run() {
    println!("🧼 Listing all launch agents and daemons...");
    log("Starting plist listing");

    let targets = vec![
        ("User LaunchAgents", "~/Library/LaunchAgents"),
        ("System LaunchAgents", "/Library/LaunchAgents"),
        ("System LaunchDaemons", "/Library/LaunchDaemons"),
    ];

    let mut plist_files: Vec<(String, PathBuf)> = Vec::new();

    for (group, raw_path) in &targets {
        let path = tilde(raw_path).to_string();
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                let plist_path = entry.path();

                // Only consider .plist files
                if plist_path.extension().and_then(|s| s.to_str()) != Some("plist") {
                    continue;
                }

                // Try to read the Label property for display purposes
                let label = run_cmd(
                    "defaults",
                    &["read", plist_path.to_str().unwrap_or(""), "Label"],
                )
                .trim()
                .to_string();

                let display_label = if !label.is_empty() {
                    format!("[{}] {}", group, label)
                } else {
                    // Fallback to filename if Label not found
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

    // Collect display labels for user selection prompt
    let options: Vec<String> = plist_files.iter().map(|(label, _)| label.clone()).collect();

    // Prompt user to select files to delete (multi-select)
    let to_delete = MultiSelect::new("Select .plist files to delete:", options.clone())
        .prompt()
        .unwrap_or_default();

    for selected in to_delete {
        if let Some((_, path)) = plist_files.iter().find(|(label, _)| label == &selected) {
            // Run `sudo rm -f` on the selected plist file
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
