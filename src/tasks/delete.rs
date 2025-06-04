use crate::logger::log;
use inquire::{MultiSelect, Select};
use once_cell::sync::Lazy;
use plist::Value;
use regex::Regex;
use serde::{Deserialize, Serialize};
use shellexpand::tilde;
use std::collections::VecDeque;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use trash;

#[derive(Serialize, Deserialize)]
struct Config {
    custom_dirs: Vec<String>,
}

static TRASHED_FILES: Lazy<Mutex<VecDeque<PathBuf>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

pub fn run() {
    println!("🗑 Deleting application and related files...");
    log("Starting app deletion");

    match prompt_user_to_select_app() {
        Ok(Some(app_name)) => {
            if let Err(e) = delete(&app_name) {
                println!("❌ Failed to delete {}: {}", app_name, e);
                log(&format!("Failed to delete {}: {}", app_name, e));
            } else {
                println!("✅ {} and related files removed.", app_name);
                log(&format!("Deleted {} and related files", app_name));
            }
        }
        Ok(None) => {
            println!("📁 No apps found in /Applications or no selection made.");
            log("No apps found or no selection made");
        }
        Err(e) => {
            println!("❌ Error selecting app: {}", e);
            log(&format!("Error selecting app: {}", e));
        }
    }
    log("Operation complete");
}

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

    app_names.sort();

    let selection = Select::new("Select an app to delete:", app_names)
        .prompt()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(Some(selection))
}

fn delete(app_name: &str) -> std::io::Result<bool> {
    let mut had_errors = false;

    if is_homebrew_app(app_name) {
        println!("🗑 Uninstalling {} via Homebrew...", app_name);
        log(&format!("Uninstalling {} via Homebrew", app_name));
        match Command::new("brew")
            .args(&["uninstall", "--cask", app_name])
            .status()
        {
            Ok(status) if status.success() => {}
            _ => {
                println!("⚠️ Failed to uninstall {} via Homebrew.", app_name);
                log(&format!("Failed to uninstall {} via Homebrew", app_name));
                had_errors = true;
            }
        }
    } else {
        let app_bundle = tilde(&format!("/Applications/{}.app", app_name)).to_string();
        let app_path = Path::new(&app_bundle);
        let bundle_id = get_bundle_identifier(app_path);
        let related_paths = find_related_files(app_name, bundle_id.as_deref());

        let path_strings: Vec<String> = related_paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let selected = MultiSelect::new(
            "Select items to delete (directories will delete all contents):",
            path_strings,
        )
        .prompt()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        if app_path.exists() {
            println!("🗑 Moving app bundle to Trash: {}", app_bundle);
            log(&format!("Moving app bundle to Trash: {}", app_bundle));
            match trash::delete(&app_path) {
                Ok(_) => {
                    let mut trashed = TRASHED_FILES.lock().unwrap();
                    trashed.push_back(app_path.to_path_buf());
                }
                Err(e) => {
                    println!("❌ Failed to move app bundle to Trash: {}", e);
                    log(&format!("Failed to move app bundle to Trash: {}", e));
                    had_errors = true;
                }
            }
        }

        for path_str in selected {
            let path = PathBuf::from(&path_str);
            println!("🗑 Moving to Trash: {}", path.display());
            log(&format!("Moving to Trash: {}", path.display()));
            match trash::delete(&path) {
                Ok(_) => {
                    let mut trashed = TRASHED_FILES.lock().unwrap();
                    trashed.push_back(path);
                }
                Err(e) => {
                    println!("⚠️ Failed to move {} to Trash: {}", path.display(), e);
                    log(&format!(
                        "Failed to move {} to Trash: {}",
                        path.display(),
                        e
                    ));
                    had_errors = true;
                }
            }
        }
    }

    Ok(!had_errors)
}

fn load_config() -> Config {
    let config_path = tilde("~/.config/myappcleaner/config.json").to_string();
    if let Ok(file) = File::open(&config_path) {
        serde_json::from_reader(file).unwrap_or(Config {
            custom_dirs: vec![],
        })
    } else {
        Config {
            custom_dirs: vec![],
        }
    }
}

fn find_related_files(app_name: &str, bundle_id: Option<&str>) -> Vec<PathBuf> {
    let mut results = Vec::new();

    let app_dir_dirs = vec![
        "~/Library/Application Support",
        "~/Library/Caches",
        "~/Library/Logs",
        "/Library/Application Support",
        "/Library/Caches",
    ];

    let file_dirs = vec!["~/Library/Preferences", "/Library/Preferences"];

    let mut search_dirs: Vec<String> = vec![
        "~/Library/Application Support",
        "~/Library/Preferences",
        "~/Library/Caches",
        "~/Library/Logs",
        "/Library/Application Support",
        "/Library/Preferences",
        "/Library/Caches",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    search_dirs.extend(load_config().custom_dirs);

    let app_name_lc = app_name.to_lowercase();
    let re_app = Regex::new(&format!(r"(?i){}", regex::escape(&app_name_lc))).unwrap();
    let re_bundle = bundle_id.map(|id| Regex::new(&format!(r"(?i){}", regex::escape(id))).unwrap());

    for dir in search_dirs {
        let expanded = PathBuf::from(tilde(&dir).to_string());

        if app_dir_dirs.contains(&dir.as_str()) || !file_dirs.contains(&dir.as_str()) {
            if expanded.exists() {
                for entry in fs::read_dir(&expanded).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if path.is_dir() {
                        let dir_name = path.file_name().unwrap().to_string_lossy();
                        if re_bundle
                            .as_ref()
                            .map_or(false, |re| re.is_match(&dir_name))
                            || re_app.is_match(&dir_name)
                        {
                            results.push(path);
                        }
                    }
                }
            }
        } else if file_dirs.contains(&dir.as_str()) {
            if expanded.exists() {
                for entry in fs::read_dir(&expanded).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "plist") {
                        let file_name = path.file_name().unwrap().to_string_lossy();
                        if re_bundle
                            .as_ref()
                            .map_or(false, |re| re.is_match(&file_name))
                            || re_app.is_match(&file_name)
                        {
                            results.push(path);
                        }
                    }
                }
            }
        }
    }

    results
}

fn get_bundle_identifier(app_path: &Path) -> Option<String> {
    let plist_path = app_path.join("Contents/Info.plist");
    if let Ok(file) = File::open(&plist_path) {
        if let Ok(plist) = Value::from_reader(file) {
            if let Some(dict) = plist.as_dictionary() {
                return dict
                    .get("CFBundleIdentifier")
                    .and_then(|v| v.as_string())
                    .map(String::from);
            }
        }
    }
    None
}

fn is_homebrew_app(app_name: &str) -> bool {
    let output = Command::new("brew")
        .args(&["list", "--cask"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    output.contains(app_name.to_lowercase().as_str())
}
