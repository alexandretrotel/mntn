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

/// User config loaded from ~/.config/myappcleaner/config.json
#[derive(Serialize, Deserialize)]
struct Config {
    custom_dirs: Vec<String>,
}

/// Global queue to track what was sent to trash.
static TRASHED_FILES: OnceLock<Mutex<VecDeque<PathBuf>>> = OnceLock::new();

fn trashed_files() -> &'static Mutex<VecDeque<PathBuf>> {
    TRASHED_FILES.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Main entry point for the app deletion process.
///
/// Guides the user through selecting an installed macOS `.app` bundle from the `/Applications` directory,
/// then deletes it along with associated files and folders (e.g., caches, preferences, logs).
///
/// The process involves:
/// - Checking if the selected app is managed by Homebrew (`brew uninstall --cask`)
/// - If not, locating associated files via name and bundle identifier matches
/// - Confirming with the user which related files to delete
/// - Moving selected files to the system Trash (non-destructive)
///
/// Logs all major events and prints user-facing status messages.
///
/// # Behavior
/// - Automatically ignores apps that cannot be found
/// - Falls back gracefully if selection fails or nothing is selected
/// - Logs and prints detailed errors if any failure occurs
///
/// # Example
/// ```no_run
/// app_delete::run();
/// ```
///
/// # Errors
/// All errors are logged and printed; no panics.
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
///
/// Uses [`inquire::Select`] to present a terminal UI prompt.
///
/// # Returns
/// - `Ok(Some(String))` if the user selects an app name
/// - `Ok(None)` if no `.app` bundles are found or the list is empty
/// - `Err(io::Error)` if directory access fails or prompt fails
///
/// # Errors
/// Converts `inquire::error::InquireError` into `std::io::Error` for easier propagation.
///
/// # Example
/// ```
/// if let Ok(Some(app_name)) = prompt_user_to_select_app() {
///     println!("User selected: {}", app_name);
/// }
/// ```
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
///
/// # Parameters
/// - `app_name`: the base name of the `.app` bundle (without `.app` extension)
///
/// # Returns
/// - `Ok(true)` if all deletions succeeded
/// - `Ok(false)` if partial errors occurred (e.g., some files failed to trash)
/// - `Err(io::Error)` if the process could not complete at all
///
/// # Process
/// 1. Checks if app is a Homebrew cask and tries to uninstall it via `brew uninstall --cask`
/// 2. Otherwise:
///     - Locates the app's `.app` bundle and moves it to the Trash
///     - Uses the bundle ID (from Info.plist) and app name to match related files
///     - Prompts the user to select which files to delete
///     - Moves selected files to the Trash
///
/// # Behavior
/// - Uses `trash::delete` for safe file removal
/// - Logs all events, including successful deletions and failures
///
/// # Example
/// ```
/// let success = delete("Visual Studio Code")?;
/// if success {
///     println!("Deleted successfully");
/// }
/// ```
fn delete(app_name: &str) -> std::io::Result<bool> {
    let mut had_errors = false;

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

/// Loads user configuration from `~/.config/myappcleaner/config.json`.
///
/// The config file contains custom directories to search for related app files.
/// This allows users to extend cleanup behavior beyond default system paths.
///
/// # Returns
/// A `Config` struct:
/// - If the file is found and parsed successfully, returns user-specified directories.
/// - If the file is missing or malformed, returns an empty `custom_dirs` vector.
///
/// # Example config file
/// ```json
/// {
///   "custom_dirs": ["/Users/username/.myapp/data"]
/// }
/// ```
///
/// # Example
/// ```
/// let config = load_config();
/// println!("{:?}", config.custom_dirs);
/// ```
fn load_config() -> Config {
    let config_path = tilde("~/.config/myappcleaner/config.json").to_string();
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
/// # Parameters
/// - `app_name`: Name of the app selected by the user (e.g., "Firefox")
/// - `bundle_id`: Optional string like `org.mozilla.firefox` extracted from the app’s Info.plist
///
/// # Returns
/// A vector of `PathBuf` entries representing related files or directories to consider deleting.
///
/// # Matching Rules
/// - For directories (like Application Support): matches if the folder name includes app name or bundle ID
/// - For `.plist` files in Preferences: matches by filename
///
/// # Example
/// ```
/// let matches = find_related_files("Firefox", Some("org.mozilla.firefox"));
/// for path in matches {
///     println!("Found: {}", path.display());
/// }
/// ```
///
/// # Errors
/// Ignores entries it cannot read and continues searching.
///
/// # Notes
/// Will skip non-existing directories silently.
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
            let name = path.file_name().unwrap_or_default().to_string_lossy();

            let matches =
                re_app.is_match(&name) || re_bundle.as_ref().map_or(false, |re| re.is_match(&name));

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
///
/// # Parameters
/// - `app_path`: Path to the `.app` bundle (e.g., `/Applications/Foo.app`)
///
/// # Returns
/// - `Some(String)` if the bundle identifier is found in the Info.plist
/// - `None` if the file is missing or cannot be parsed
///
/// # Example
/// ```
/// let id = get_bundle_identifier(Path::new("/Applications/Foo.app"));
/// if let Some(bundle_id) = id {
///     println!("Bundle ID: {}", bundle_id);
/// }
/// ```
///
/// # Notes
/// Looks for `Contents/Info.plist` inside the `.app` directory.
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
///
/// # Parameters
/// - `app_name`: The app’s base name (e.g., "firefox")
///
/// # Returns
/// - `true` if the app name appears in `brew list --cask`
/// - `false` otherwise (either not installed or not a cask)
///
/// # Behavior
/// - Runs `brew list --cask` and checks for a case-insensitive match
/// - Falls back to `false` on any command failure
///
/// # Example
/// ```
/// if is_homebrew_app("firefox") {
///     println!("Firefox is a Homebrew Cask app");
/// }
/// ```
fn is_homebrew_app(app_name: &str) -> bool {
    Command::new("brew")
        .args(&["list", "--cask"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
        .to_lowercase()
        .contains(&app_name.to_lowercase())
}

/// Helper function to log and display an error in a consistent format.
///
/// # Parameters
/// - `context`: Describes what the error was trying to do (e.g., "Deleting app")
/// - `error`: The error itself (or optional `None`)
///
/// # Example
/// ```
/// prompt_error("Failed to uninstall via Homebrew", Some(err));
/// ```
///
/// # Side Effects
/// - Prints a red ❌-style message to the console
/// - Logs the error via the custom `log` function
fn prompt_error(context: &str, error: impl std::fmt::Debug) {
    println!("❌ {}: {:?}", context, error);
    log(&format!("{}: {:?}", context, error));
}
