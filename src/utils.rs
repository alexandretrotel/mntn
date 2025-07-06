use std::{
    fs::{self},
    path::{Path, PathBuf},
    process::Command,
};

pub fn run_cmd(cmd: &str, args: &[&str]) -> String {
    let output = Command::new(cmd).args(args).output();

    match output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(_) => String::new(),
    }
}

pub fn calculate_dir_size(path: &Path) -> Option<u64> {
    if path.is_symlink() {
        return Some(0);
    }
    let mut size = 0;
    if path.is_file() {
        size += fs::metadata(path).ok()?.len();
    } else if path.is_dir() {
        let entries = fs::read_dir(path).ok()?;
        for entry in entries {
            let entry = entry.ok()?;
            let entry_path = entry.path();
            if entry_path.is_symlink() {
                continue;
            }
            size += calculate_dir_size(&entry_path).unwrap_or(0);
        }
    }
    Some(size)
}

pub fn bytes_to_human_readable(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

pub fn get_vscode_settings_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let vscode_path = home_dir.join("Library/Application Support/Code/User/settings.json");
    if vscode_path.exists() {
        Some(vscode_path)
    } else {
        None
    }
}

pub fn get_vscode_keybindings_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let vscode_path = home_dir.join("Library/Application Support/Code/User/keybindings.json");
    if vscode_path.exists() {
        Some(vscode_path)
    } else {
        None
    }
}

pub fn get_cursor_settings_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let cursor_path = home_dir.join("Library/Application Support/Cursor/User/settings.json");
    if cursor_path.exists() {
        Some(cursor_path)
    } else {
        None
    }
}

pub fn get_cursor_keybindings_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let cursor_path = home_dir.join("Library/Application Support/Cursor/User/keybindings.json");
    if cursor_path.exists() {
        Some(cursor_path)
    } else {
        None
    }
}

pub fn get_iterm_preferences_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let iterm_path = home_dir.join("Library/Preferences/com.googlecode.iterm2.plist");
    if iterm_path.exists() {
        Some(iterm_path)
    } else {
        None
    }
}

pub fn get_ghostty_config_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let ghostty_path = home_dir.join("Library/Application Support/com.mitchellh.ghostty/config");
    if ghostty_path.exists() {
        Some(ghostty_path)
    } else {
        None
    }
}
