use crate::cli::PurgeArgs;
use crate::logger::log;
use crate::utils::system::run_cmd;
use inquire::MultiSelect;
use std::fs;
use std::path::PathBuf;

/// Represents a directory target for scanning plist files
#[derive(Debug, Clone)]
struct DirectoryTarget {
    name: &'static str,
    path: &'static str,
    is_system: bool,
}

/// Represents a found plist file with metadata
#[derive(Debug)]
struct PlistFile {
    display_label: String,
    path: PathBuf,
    is_system: bool,
}

/// Lists all macOS LaunchAgents and LaunchDaemons `.plist` files from standard system directories,
/// prompts the user to select which ones to delete, and deletes the selected files.
pub fn run(args: PurgeArgs) {
    println!("🧼 Listing all launch agents and daemons...");
    log("Starting plist listing");

    let targets = get_directory_targets(args.system);
    let plist_files = scan_plist_files(&targets);

    if plist_files.is_empty() {
        println!("📁 No .plist files found.");
        log("No .plist files found.");
        return;
    }

    // Collect display labels for user selection prompt
    let options: Vec<String> = plist_files
        .iter()
        .map(|f| f.display_label.clone())
        .collect();

    // Prompt user to select files to delete (multi-select)
    let action_verb = if args.dry_run {
        "preview deletion for"
    } else {
        "delete"
    };
    let prompt_message = format!("Select .plist files to {}:", action_verb);

    let to_delete = MultiSelect::new(&prompt_message, options.clone())
        .prompt()
        .unwrap_or_default();

    if args.dry_run {
        println!("🔍 Dry run - would delete the following files:");
        for selected in to_delete {
            if let Some(plist_file) = plist_files.iter().find(|f| f.display_label == selected) {
                println!("  - {}", plist_file.path.display());
                log(&format!("Would delete: {}", plist_file.path.display()));
            }
        }
        println!("✅ Dry run complete. No files were actually deleted.");
        log("Dry run complete");
    } else {
        for selected in to_delete {
            if let Some(plist_file) = plist_files.iter().find(|f| f.display_label == selected) {
                delete_plist_file(&plist_file.path, plist_file.is_system);
                log(&format!("Deleted: {}", plist_file.path.display()));
            }
        }
        log("Plist deletion complete");
        println!("✅ Selected files deleted.");
    }
}

/// Returns the directory targets to scan based on the system flag
fn get_directory_targets(include_system: bool) -> Vec<DirectoryTarget> {
    let mut targets = vec![DirectoryTarget {
        name: "User LaunchAgents",
        path: "~/Library/LaunchAgents",
        is_system: false,
    }];

    if include_system {
        targets.push(DirectoryTarget {
            name: "System LaunchAgents",
            path: "/Library/LaunchAgents",
            is_system: true,
        });
        targets.push(DirectoryTarget {
            name: "System LaunchDaemons",
            path: "/Library/LaunchDaemons",
            is_system: true,
        });
    }

    targets
}

/// Scans the specified directories for .plist files and returns them with metadata
fn scan_plist_files(targets: &[DirectoryTarget]) -> Vec<PlistFile> {
    let mut plist_files = Vec::new();

    for target in targets {
        let path = shellexpand::tilde(target.path).to_string();
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                let plist_path = entry.path();

                // Only consider .plist files
                if plist_path.extension().and_then(|s| s.to_str()) != Some("plist") {
                    continue;
                }

                let display_label = get_plist_display_label(target.name, &plist_path);

                plist_files.push(PlistFile {
                    display_label,
                    path: plist_path,
                    is_system: target.is_system,
                });
            }
        }
    }

    plist_files
}

/// Gets a friendly display label for a plist file
fn get_plist_display_label(group_name: &str, plist_path: &PathBuf) -> String {
    let label_result = run_cmd(
        "defaults",
        &["read", plist_path.to_str().unwrap_or(""), "Label"],
    );

    let label = match label_result {
        Ok(output) => output.trim().to_string(),
        Err(_) => String::new(),
    };

    if !label.is_empty() {
        format!("[{}] {}", group_name, label)
    } else {
        let fallback = plist_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown.plist");
        format!("[{}] {}", group_name, fallback)
    }
}

/// Attempts to delete a plist file, trying fs::remove_file first, then sudo if needed
fn delete_plist_file(path: &PathBuf, is_system_file: bool) {
    match fs::remove_file(path) {
        Ok(_) => {
            println!("🗑️  Deleted: {}", path.display());
        }
        Err(_) => {
            if is_system_file {
                println!("🔐 Requires elevated privileges, using sudo...");
                let result = std::process::Command::new("sudo")
                    .arg("rm")
                    .arg("-f")
                    .arg(path)
                    .status();

                match result {
                    Ok(status) if status.success() => {
                        println!("🗑️  Deleted with sudo: {}", path.display());
                    }
                    _ => {
                        println!("❌ Failed to delete: {}", path.display());
                        log(&format!("Failed to delete: {}", path.display()));
                    }
                }
            } else {
                println!("❌ Failed to delete: {}", path.display());
                log(&format!("Failed to delete: {}", path.display()));
            }
        }
    }
}
