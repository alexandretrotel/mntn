use std::path::PathBuf;

/// Returns the path to the VSCode user settings file on macOS, if it exists.
///
/// Looks for `~/Library/Application Support/Code/User/settings.json`.
///
/// # Returns
///
/// * `Some(PathBuf)` if the settings file exists.
/// * `None` if the file does not exist or the home directory cannot be determined.
///
/// # Note
///
/// This path is specific to macOS.
///
/// # Examples
///
/// ```
/// use mntn::utils::app_paths::get_vscode_settings_path;
///
/// if let Some(path) = get_vscode_settings_path() {
///     println!("VSCode settings at {:?}", path);
/// }
/// ```
pub fn get_vscode_settings_path() -> Option<PathBuf> {
    let data_local_dir = dirs_next::data_local_dir()?;
    let vscode_path = data_local_dir.join("Code/User/settings.json");
    if vscode_path.exists() {
        Some(vscode_path)
    } else {
        None
    }
}

/// Returns the path to the VSCode user keybindings file on macOS, if it exists.
///
/// Looks for `~/Library/Application Support/Code/User/keybindings.json`.
///
/// # Returns
///
/// * `Some(PathBuf)` if the keybindings file exists.
/// * `None` if the file does not exist or the home directory cannot be determined.
///
/// # Note
///
/// This path is specific to macOS.
///
/// # Examples
///
/// ```
/// use mntn::utils::app_paths::get_vscode_keybindings_path;
///
/// if let Some(path) = get_vscode_keybindings_path() {
///     println!("VSCode keybindings at {:?}", path);
/// }
/// ```
pub fn get_vscode_keybindings_path() -> Option<PathBuf> {
    let data_local_dir = dirs_next::data_local_dir()?;
    let vscode_path = data_local_dir.join("Code/User/keybindings.json");
    if vscode_path.exists() {
        Some(vscode_path)
    } else {
        None
    }
}

/// Returns the path to the Ghostty configuration file on macOS, if it exists.
///
/// Looks for `~/Library/Application Support/com.mitchellh.ghostty/config`.
///
/// # Returns
///
/// * `Some(PathBuf)` if the config file exists.
/// * `None` if the file does not exist or the home directory cannot be determined.
///
/// # Note
///
/// This path is specific to macOS.
///
/// # Examples
///
/// ```
/// use mntn::utils::app_paths::get_ghostty_config_path;
///
/// if let Some(path) = get_ghostty_config_path() {
///     println!("Ghostty config at {:?}", path);
/// }
/// ```
pub fn get_ghostty_config_path() -> Option<PathBuf> {
    let data_local_dir = dirs_next::data_local_dir()?;
    let ghostty_path = data_local_dir.join("com.mitchellh.ghostty/config");
    if ghostty_path.exists() {
        Some(ghostty_path)
    } else {
        None
    }
}
