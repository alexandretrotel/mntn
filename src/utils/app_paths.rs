use std::path::PathBuf;

/// Resolves an application-specific path inside the local data directory,
fn resolve_config_path(relative: &str) -> Option<PathBuf> {
    let base = dirs_next::config_dir()?;
    let path = base.join(relative);
    path.exists().then_some(path)
}

/// Returns the path to the VSCode user settings file, if it exists.
pub fn get_vscode_settings_path() -> Option<PathBuf> {
    resolve_config_path("Code/User/settings.json")
}

/// Returns the path to the VSCode user keybindings file, if it exists.
pub fn get_vscode_keybindings_path() -> Option<PathBuf> {
    resolve_config_path("Code/User/keybindings.json")
}

/// Returns the path to the Ghostty configuration file, if it exists.
pub fn get_ghostty_config_path() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        resolve_config_path("com.mitchellh.ghostty/config")
    } else {
        resolve_config_path("ghostty/config")
    }
}
