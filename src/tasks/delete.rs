use crate::logger::log;
use inquire::Select;
use regex::Regex;
use shellexpand::tilde;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

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

    log("App deletion complete");
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

    let app_bundle = tilde(&format!("/Applications/{}.app", app_name)).to_string();
    let app_path = Path::new(&app_bundle);

    if !app_path.exists() {
        println!("⚠️ App bundle not found at {}", app_bundle);
        log(&format!("App bundle not found at {}", app_bundle));
    } else {
        println!("🗑 Removing app bundle: {}", app_bundle);
        log(&format!("Removing app bundle: {}", app_bundle));
        match fs::remove_dir_all(&app_path) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                println!("🔐 Permission denied. Retrying with sudo...");
                log(&format!(
                    "Permission denied removing {}. Retrying with sudo...",
                    app_bundle
                ));
                if !sudo_rm(&app_bundle) {
                    println!("❌ Failed to remove app bundle with sudo.");
                    log(&format!("Failed to remove {} with sudo", app_bundle));
                    had_errors = true;
                }
            }
            Err(e) => {
                println!("❌ Failed to remove app bundle: {}", e);
                log(&format!("Failed to remove app bundle: {}", e));
                had_errors = true;
            }
        }
    }

    let related_paths = find_related_files(app_name);
    for path in &related_paths {
        println!("🗑 Removing related file: {}", path.display());
        log(&format!("Removing related file: {}", path.display()));
        let result = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };

        match result {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                println!("🔐 Permission denied. Retrying with sudo...");
                log(&format!(
                    "Permission denied removing {}. Retrying with sudo...",
                    path.display()
                ));
                if !sudo_rm(&path.to_string_lossy()) {
                    println!("❌ Failed to remove {} with sudo.", path.display());
                    log(&format!("Failed to remove {} with sudo", path.display()));
                    had_errors = true;
                }
            }
            Err(e) => {
                println!("⚠️ Failed to remove {}: {}", path.display(), e);
                log(&format!("Failed to remove {}: {}", path.display(), e));
                had_errors = true;
            }
        }
    }

    Ok(had_errors)
}

fn find_related_files(app_name: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let search_dirs = vec![
        "~/Library/Application Support",
        "~/Library/Preferences",
        "~/Library/Caches",
        "~/Library/Logs",
        "/Library/Application Support",
        "/Library/Preferences",
        "/Library/Caches",
    ];

    let app_name_lc = app_name.to_lowercase();
    let re = Regex::new(&format!(r"(?i){}", regex::escape(&app_name_lc))).unwrap();

    for dir in search_dirs {
        let expanded = PathBuf::from(tilde(dir).to_string());
        for entry in WalkDir::new(&expanded).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.to_string_lossy().to_lowercase().contains(&app_name_lc)
                || re.is_match(&path.to_string_lossy())
            {
                results.push(path.to_path_buf());
            }
        }
    }

    results
}

fn sudo_rm(path: &str) -> bool {
    match Command::new("sudo").arg("rm").arg("-rf").arg(path).status() {
        Ok(status) if status.success() => true,
        _ => false,
    }
}
