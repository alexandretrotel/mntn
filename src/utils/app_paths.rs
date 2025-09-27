use std::path::PathBuf;

/// Resolves an application-specific path inside the local data directory,
/// returning `Some(PathBuf)` if the file exists, otherwise `None`.
fn resolve_config_path(relative: &str) -> Option<PathBuf> {
    let base = dirs_next::data_local_dir()?;
    let path = base.join(relative);
    path.exists().then_some(path)
}

/// Returns the path to the VSCode user settings file on macOS, if it exists.
///
/// Path: `~/Library/Application Support/Code/User/settings.json`
pub fn get_vscode_settings_path() -> Option<PathBuf> {
    resolve_config_path("Code/User/settings.json")
}

/// Returns the path to the VSCode user keybindings file on macOS, if it exists.
///
/// Path: `~/Library/Application Support/Code/User/keybindings.json`
pub fn get_vscode_keybindings_path() -> Option<PathBuf> {
    resolve_config_path("Code/User/keybindings.json")
}

/// Returns the path to the Ghostty configuration file on macOS, if it exists.
///
/// Path: `~/Library/Application Support/com.mitchellh.ghostty/config`
pub fn get_ghostty_config_path() -> Option<PathBuf> {
    resolve_config_path("com.mitchellh.ghostty/config")
}
