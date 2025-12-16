use crate::cli::DeleteArgs;
use crate::logger::log;
use crate::utils::paths::get_base_dirs;
use inquire::{MultiSelect, Select};
use plist::Value;
use regex::Regex;
use serde::{Deserialize, Serialize};
use shellexpand::tilde;
use std::collections::VecDeque;
use std::fs::{self, File};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use trash;

/// User config loaded from ~/.config/mntn/config.json
#[derive(Serialize, Deserialize)]
struct Config {
    custom_dirs: Vec<String>,
}

/// Represents directories and files that need to be processed
#[derive(Debug)]
struct FilesToProcess {
    user_files: Vec<PathBuf>,
    system_files: Vec<PathBuf>,
}

/// Global queue to track what was sent to trash.
static TRASHED_FILES: OnceLock<Mutex<VecDeque<PathBuf>>> = OnceLock::new();

fn trashed_files() -> &'static Mutex<VecDeque<PathBuf>> {
    TRASHED_FILES.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Guides the user through selecting an installed macOS `.app` bundle from the `/Applications` directory,
/// then deletes it along with associated files and folders (e.g., caches, preferences, logs).
///
/// The process involves:
/// - Checking if the selected app is managed by Homebrew (`brew uninstall --cask`)
/// - If not, locating associated files via name and bundle identifier matches
/// - Confirming with the user which related files to delete
/// - Moving selected files to the system Trash (non-destructive) or permanently deleting them
pub fn run(args: DeleteArgs) {
    if args.dry_run {
        println!("🔍 Running in dry-run mode - no files will be deleted");
    } else if args.permanent {
        println!("🗑 Permanently deleting application and related files...");
    } else {
        println!("🗑 Moving application and related files to trash...");
    }
    log(&format!(
        "Starting app deletion with args: dry_run={}, permanent={}",
        args.dry_run, args.permanent
    ));

    match prompt_user_to_select_app() {
        Ok(Some(app_name)) => match delete(&app_name, &args) {
            Ok(true) => {
                if args.dry_run {
                    println!("✅ {} and related files would be removed.", app_name);
                } else {
                    println!("✅ {} and related files removed.", app_name);
                }
                log(&format!("Processed {} and related files", app_name));
            }
            Ok(false) => {
                if args.dry_run {
                    println!(
                        "⚠️ {} would be partially deleted (some issues detected).",
                        app_name
                    );
                } else {
                    println!(
                        "⚠️ {} was partially deleted (some errors occurred).",
                        app_name
                    );
                }
                log(&format!("Partial processing for {}", app_name));
            }
            Err(e) => prompt_error(&format!("Failed to process {}", app_name), e),
        },
        Ok(None) => {
            println!("📁 No apps found or no selection made.");
            log("No apps found or no selection made");
        }
        Err(e) => prompt_error("Error selecting app", e),
    }

    log("Operation complete");
}

/// Prompts the user to choose an installed application from `/Applications`.
///
/// Filters for files ending in `.app`, strips the extension, and sorts the list for display.
fn prompt_user_to_select_app() -> std::io::Result<Option<String>> {
    let apps_dir = Path::new("/Applications");
    let mut app_names = vec![];

    for entry in fs::read_dir(apps_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "app")
            && let Some(name) = path.file_stem().and_then(|s| s.to_str())
        {
            app_names.push(name.to_string());
        }
    }

    if app_names.is_empty() {
        return Ok(None);
    }

    let selection = Select::new("Select an app to delete:", app_names)
        .prompt()
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    Ok(Some(selection))
}

/// Deletes a selected app and associated files by either:
/// - Uninstalling via Homebrew if applicable
/// - Moving its `.app` bundle and related files to the Trash or permanently deleting them
fn delete(app_name: &str, args: &DeleteArgs) -> std::io::Result<bool> {
    let mut had_errors = false;

    // Check if the app is managed by Homebrew
    if is_homebrew_app(app_name) {
        if args.dry_run {
            println!("[DRY RUN] Would uninstall {} via Homebrew", app_name);
        } else {
            println!("🗑️ Uninstalling {} via Homebrew...", app_name);
            log(&format!("Uninstalling {} via Homebrew", app_name));
            let status = Command::new("brew")
                .args(["uninstall", "--cask", app_name])
                .status();

            if !matches!(status, Ok(s) if s.success()) {
                had_errors = true;
                prompt_error(
                    &format!("Failed to uninstall {} via Homebrew", app_name),
                    format!("{:?}", status.err()),
                );
            }
        }
    }

    // Proceed with manual deletion of app bundle and related files
    let app_path = PathBuf::from(format!("/Applications/{}.app", app_name));
    let bundle_id = get_bundle_identifier(&app_path);
    let files_to_process = find_related_files_categorized(app_name, bundle_id.as_deref());

    // Combine all files for selection
    let mut all_files = files_to_process.user_files.clone();
    all_files.extend(files_to_process.system_files.clone());

    let options: Vec<String> = all_files
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_system = files_to_process.system_files.contains(p);
            let prefix = if is_system { "[SYSTEM] " } else { "[USER] " };
            format!("{}{}: {}", prefix, i, p.display())
        })
        .collect();

    if options.is_empty() && !app_path.exists() {
        println!("📁 No related files found for {}", app_name);
        return Ok(true);
    }

    let selected = if !options.is_empty() {
        MultiSelect::new("Select items to delete:", options)
            .prompt()
            .map_err(|e| std::io::Error::other(e.to_string()))?
    } else {
        Vec::new()
    };

    // Process app bundle
    if app_path.exists() {
        had_errors |= !process_file(&app_path, args, true)?;
    }

    // Process selected files
    let mut system_files_to_delete = Vec::new();
    let mut user_files_to_delete = Vec::new();

    for selected_item in selected {
        let path_str = if selected_item.starts_with("[SYSTEM] ") {
            selected_item.strip_prefix("[SYSTEM] ").unwrap()
        } else {
            selected_item.strip_prefix("[USER] ").unwrap()
        };

        let path = PathBuf::from(path_str);

        if files_to_process.system_files.contains(&path) {
            system_files_to_delete.push(path);
        } else {
            user_files_to_delete.push(path);
        }
    }

    // Process user files first (no sudo needed)
    for path in user_files_to_delete {
        had_errors |= !process_file(&path, args, false)?;
    }

    // Process system files with sudo if needed
    if !system_files_to_delete.is_empty() && !args.dry_run {
        if args.permanent {
            println!("🔐 System files require sudo for permanent deletion...");
        } else {
            println!("🔐 System files require sudo for moving to trash...");
        }
    }

    for path in system_files_to_delete {
        had_errors |= !process_file(&path, args, true)?;
    }

    Ok(!had_errors)
}

/// Loads user configuration from `~/.config/mntn/config.json`.
///
/// The config file contains custom directories to search for related app files.
/// This allows users to extend cleanup behavior beyond default system paths.
fn load_config() -> Config {
    let base_dirs = get_base_dirs();
    let config_dir = base_dirs.config_dir();
    let config_path = config_dir.join("mntn/config.json");
    File::open(&config_path)
        .ok()
        .and_then(|file| serde_json::from_reader(file).ok())
        .unwrap_or(Config {
            custom_dirs: vec![],
        })
}

/// Searches for files and folders related to a given app name and optional bundle ID.
///
/// Uses a regex match against:
/// - Directory and file names inside known locations (Caches, Logs, Preferences, etc.)
/// - User-configured custom paths from the config file
///
/// Returns files categorized by whether they need sudo (system) or not (user)
fn find_related_files_categorized(app_name: &str, bundle_id: Option<&str>) -> FilesToProcess {
    let mut user_files = Vec::new();
    let mut system_files = Vec::new();

    let app_name_lc = app_name.to_lowercase();
    let re_app = Regex::new(&format!(r"(?i){}", regex::escape(&app_name_lc))).unwrap();
    let re_bundle = bundle_id.map(|id| Regex::new(&format!(r"(?i){}", regex::escape(id))).unwrap());

    let base_dirs = get_base_dirs();
    let home_dir = base_dirs.home_dir();
    let data_dir = base_dirs.data_dir();
    let cache_dir = base_dirs.cache_dir();

    let user_app_dirs = vec![
        data_dir.to_path_buf(),
        cache_dir.to_path_buf(),
        home_dir.join("Library/Logs"),
    ];
    let user_file_dirs = vec![home_dir.join("Library/Preferences")];

    let system_app_dirs = vec![
        PathBuf::from("/Library/Application Support"),
        PathBuf::from("/Library/Caches"),
    ];
    let system_file_dirs = vec![PathBuf::from("/Library/Preferences")];

    // Process user directories
    for (dirs, is_app_dir) in [(user_app_dirs, true), (user_file_dirs, false)] {
        for dir in dirs {
            process_directory(&dir, &re_app, &re_bundle, is_app_dir, &mut user_files);
        }
    }

    // Process system directories
    for (dirs, is_app_dir) in [(system_app_dirs, true), (system_file_dirs, false)] {
        for dir in dirs {
            process_directory(&dir, &re_app, &re_bundle, is_app_dir, &mut system_files);
        }
    }

    // Process custom directories from config (treat as user directories by default)
    let config = load_config();
    for dir in config.custom_dirs {
        let dir_path = PathBuf::from(tilde(&dir).to_string());
        process_directory(&dir_path, &re_app, &re_bundle, true, &mut user_files);
    }

    FilesToProcess {
        user_files,
        system_files,
    }
}

/// Helper function to process a single directory and add matching files to results
fn process_directory(
    dir: &PathBuf,
    re_app: &Regex,
    re_bundle: &Option<Regex>,
    is_app_dir: bool,
    results: &mut Vec<PathBuf>,
) {
    if !dir.exists() {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };

    for entry in entries {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default();

        let matches = name.to_str().is_some_and(|name_str| {
            re_app.is_match(name_str) || re_bundle.as_ref().is_some_and(|re| re.is_match(name_str))
        });

        if matches
            && ((is_app_dir && path.is_dir())
                || (!is_app_dir && path.extension().is_some_and(|ext| ext == "plist")))
        {
            results.push(path);
        }
    }
}

/// Extracts the `CFBundleIdentifier` from an app’s `Info.plist` file.
///
/// Used to locate related system files using a more reliable identifier than the name alone.
fn get_bundle_identifier(app_path: &Path) -> Option<String> {
    let plist_path = app_path.join("Contents/Info.plist");
    File::open(&plist_path)
        .ok()
        .and_then(|file| Value::from_reader(file).ok())
        .and_then(|plist| {
            plist
                .as_dictionary()?
                .get("CFBundleIdentifier")?
                .as_string()
                .map(|s| s.to_owned())
        })
}

/// Determines whether an app is installed via Homebrew Cask.
fn is_homebrew_app(app_name: &str) -> bool {
    let output = Command::new("brew").args(["list", "--cask"]).output();

    let stdout = match output {
        Ok(o) => match String::from_utf8(o.stdout) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Invalid UTF-8 output from brew: {}", e);
                return false;
            }
        },
        Err(e) => {
            eprintln!("Failed to run brew: {}", e);
            return false;
        }
    };

    stdout.to_lowercase().contains(&app_name.to_lowercase())
}

/// Processes a single file or directory for deletion
fn process_file(path: &Path, args: &DeleteArgs, needs_sudo: bool) -> std::io::Result<bool> {
    if args.dry_run {
        let action = if args.permanent {
            "permanently delete"
        } else {
            "move to trash"
        };
        let sudo_prefix = if needs_sudo { "[SUDO] " } else { "" };
        println!("🔍 Would {}{}: {}", sudo_prefix, action, path.display());
        log(&format!(
            "Would {}{}: {}",
            sudo_prefix,
            action,
            path.display()
        ));
        return Ok(true);
    }

    if args.permanent {
        if needs_sudo {
            println!("🗑 [SUDO] Permanently deleting: {}", path.display());
            log(&format!(
                "Permanently deleting with sudo: {}",
                path.display()
            ));

            let status = if path.is_dir() {
                match fs::remove_dir_all(path) {
                    Ok(()) => Ok(std::process::ExitStatus::from_raw(0)),
                    Err(_) => Command::new("sudo").args(["rm", "-rf"]).arg(path).status(),
                }
            } else {
                match fs::remove_file(path) {
                    Ok(()) => Ok(std::process::ExitStatus::from_raw(0)),
                    Err(_) => Command::new("sudo").args(["rm", "-f"]).arg(path).status(),
                }
            };

            match status {
                Ok(s) if s.success() => Ok(true),
                _ => {
                    prompt_error(
                        &format!("Failed to permanently delete {}", path.display()),
                        "sudo rm failed",
                    );
                    Ok(false)
                }
            }
        } else {
            println!("🗑 Permanently deleting: {}", path.display());
            log(&format!("Permanently deleting: {}", path.display()));

            let result = if path.is_dir() {
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            };

            match result {
                Ok(()) => Ok(true),
                Err(e) => {
                    prompt_error(
                        &format!("Failed to permanently delete {}", path.display()),
                        e,
                    );
                    Ok(false)
                }
            }
        }
    } else {
        let action_desc = if needs_sudo {
            "[SUDO] Moving to trash"
        } else {
            "Moving to trash"
        };
        println!("🗑 {}: {}", action_desc, path.display());
        log(&format!("{}: {}", action_desc, path.display()));

        match trash::delete(path) {
            Ok(()) => {
                trashed_files()
                    .lock()
                    .unwrap()
                    .push_back(path.to_path_buf());
                Ok(true)
            }
            Err(e) => {
                prompt_error(&format!("Failed to move {} to trash", path.display()), e);
                Ok(false)
            }
        }
    }
}

/// Helper function to log and display an error in a consistent format.
fn prompt_error(context: &str, error: impl std::fmt::Debug) {
    println!("❌ {}: {:?}", context, error);
    log(&format!("{}: {:?}", context, error));
}
