use crate::logger::log;
use inquire::{MultiSelect, Select};
use plist::Value;
use regex::Regex;
use serde::{Deserialize, Serialize};
use shellexpand::tilde;
use std::collections::VecDeque;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use trash;

/// User config loaded from ~/.config/mntn/config.json
#[derive(Serialize, Deserialize)]
struct Config {
    custom_dirs: Vec<String>,
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
/// - Moving selected files to the system Trash (non-destructive)
pub fn run() {
    println!("🗑 Deleting application and related files...");
    log("Starting app deletion");

    match prompt_user_to_select_app() {
        Ok(Some(app_name)) => match delete(&app_name) {
            Ok(true) => {
                println!("✅ {} and related files removed.", app_name);
                log(&format!("Deleted {} and related files", app_name));
            }
            Ok(false) => {
                println!(
                    "⚠️ {} was partially deleted (some errors occurred).",
                    app_name
                );
                log(&format!("Partial deletion for {}", app_name));
            }
            Err(e) => prompt_error(&format!("Failed to delete {}", app_name), e),
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
    let binding = tilde("/Applications").to_string();
    let apps_dir = Path::new(&binding);
    let mut app_names = vec![];

    for entry in fs::read_dir(apps_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "app") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                app_names.push(name.to_string());
            }
        }
    }

    if app_names.is_empty() {
        return Ok(None);
    }

    let selection = Select::new("Select an app to delete:", app_names)
        .prompt()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(Some(selection))
}

/// Deletes a selected app and associated files by either:
/// - Uninstalling via Homebrew if applicable
/// - Moving its `.app` bundle and related files to the Trash
fn delete(app_name: &str) -> std::io::Result<bool> {
    let mut had_errors = false;

    // Check if the app is managed by Homebrew
    if is_homebrew_app(app_name) {
        println!("🗑 Uninstalling {} via Homebrew...", app_name);
        log(&format!("Uninstalling {} via Homebrew", app_name));
        let status = Command::new("brew")
            .args(&["uninstall", "--cask", app_name])
            .status();

        if !matches!(status, Ok(s) if s.success()) {
            had_errors = true;
            prompt_error(
                &format!("Failed to uninstall {} via Homebrew", app_name),
                format!("{:?}", status.err()),
            );
        }
    }

    // Proceed with manual deletion of app bundle and related files
    let app_path = PathBuf::from(tilde(&format!("/Applications/{}.app", app_name)).to_string());
    let bundle_id = get_bundle_identifier(&app_path);
    let related_paths = find_related_files(app_name, bundle_id.as_deref());

    let options: Vec<String> = related_paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    let selected = MultiSelect::new("Select items to delete:", options)
        .prompt()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    if app_path.exists() {
        println!("🗑 Moving app bundle to Trash: {}", app_path.display());
        log(&format!(
            "Moving app bundle to Trash: {}",
            app_path.display()
        ));
        if let Err(e) = trash::delete(&app_path) {
            had_errors = true;
            prompt_error("Failed to move app bundle to Trash", Some(e));
        } else {
            trashed_files().lock().unwrap().push_back(app_path);
        }
    }

    for path_str in selected {
        let path = PathBuf::from(&path_str);
        println!("🗑 Moving to Trash: {}", path.display());
        log(&format!("Moving to Trash: {}", path.display()));

        if let Err(e) = trash::delete(&path) {
            had_errors = true;
            prompt_error(
                &format!("Failed to move {} to Trash", path.display()),
                Some(e),
            );
        } else {
            trashed_files().lock().unwrap().push_back(path);
        }
    }

    Ok(!had_errors)
}

/// Loads user configuration from `~/.config/mntn/config.json`.
///
/// The config file contains custom directories to search for related app files.
/// This allows users to extend cleanup behavior beyond default system paths.
fn load_config() -> Config {
    let config_path = tilde("~/.config/mntn/config.json").to_string();
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
fn find_related_files(app_name: &str, bundle_id: Option<&str>) -> Vec<PathBuf> {
    let mut results = Vec::new();

    let app_name_lc = app_name.to_lowercase();
    let re_app = Regex::new(&format!(r"(?i){}", regex::escape(&app_name_lc))).unwrap();
    let re_bundle = bundle_id.map(|id| Regex::new(&format!(r"(?i){}", regex::escape(id))).unwrap());

    let app_dirs = vec![
        "~/Library/Application Support",
        "~/Library/Caches",
        "~/Library/Logs",
        "/Library/Application Support",
        "/Library/Caches",
    ];
    let file_dirs = vec!["~/Library/Preferences", "/Library/Preferences"];

    let mut search_dirs: Vec<String> = app_dirs
        .iter()
        .chain(&file_dirs)
        .map(|s| s.to_string())
        .collect();
    search_dirs.extend(load_config().custom_dirs);

    for dir in search_dirs {
        let expanded = PathBuf::from(tilde(&dir).to_string());
        if !expanded.exists() {
            continue;
        }

        let entries = match fs::read_dir(&expanded) {
            Ok(e) => e.filter_map(Result::ok).collect::<Vec<_>>(),
            Err(_) => Vec::new(),
        };

        for entry in entries {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default();

            let matches = name.to_str().map_or(false, |name_str| {
                re_app.is_match(name_str)
                    || re_bundle.as_ref().map_or(false, |re| re.is_match(name_str))
            });

            if matches {
                if (app_dirs.contains(&dir.as_str()) && path.is_dir())
                    || (file_dirs.contains(&dir.as_str())
                        && path.extension().map_or(false, |ext| ext == "plist"))
                {
                    results.push(path);
                }
            }
        }
    }

    results
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
    let output = Command::new("brew").args(&["list", "--cask"]).output();

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

/// Helper function to log and display an error in a consistent format.
fn prompt_error(context: &str, error: impl std::fmt::Debug) {
    println!("❌ {}: {:?}", context, error);
    log(&format!("{}: {:?}", context, error));
}
