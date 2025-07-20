use std::{
    fs::{self},
    path::{Path, PathBuf},
    process::Command,
};

/// Runs a system command with the given arguments and returns its standard output as a `String`.
///
/// If the command fails to run, returns an empty string.
///
/// # Arguments
///
/// * `cmd` - The command to run (e.g., "ls", "echo").
/// * `args` - A slice of argument strings to pass to the command.
///
/// # Examples
///
/// ```
/// let output = run_cmd("echo", &["hello"]);
/// assert_eq!(output.trim(), "hello");
/// ```
pub fn run_cmd(cmd: &str, args: &[&str]) -> String {
    let output = Command::new(cmd).args(args).output();

    match output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(_) => String::new(),
    }
}

/// Recursively calculates the total size in bytes of the given directory or file path.
///
/// Symlinks are ignored and contribute zero to the total size to avoid cycles.
///
/// Returns `None` if the path metadata cannot be accessed or read.
///
/// # Arguments
///
/// * `path` - The file or directory path to measure.
///
/// # Returns
///
/// * `Some(size_in_bytes)` if successful.
/// * `None` if the path does not exist or cannot be accessed.
///
/// # Examples
///
/// ```
/// let size = calculate_dir_size(Path::new("/some/path")).unwrap_or(0);
/// println!("Size: {} bytes", size);
/// ```
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

/// Converts a byte count into a human-readable string with units (bytes, KB, MB, GB).
///
/// Uses base 1024 for unit conversion.
///
/// # Arguments
///
/// * `bytes` - The number of bytes to convert.
///
/// # Returns
///
/// A formatted string representing the size in an appropriate unit with two decimal places.
///
/// # Examples
///
/// ```
/// assert_eq!(bytes_to_human_readable(1024), "1.00 KB");
/// assert_eq!(bytes_to_human_readable(500), "500 bytes");
/// ```
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
/// if let Some(path) = get_vscode_settings_path() {
///     println!("VSCode settings at {:?}", path);
/// }
/// ```
pub fn get_vscode_settings_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let vscode_path = home_dir.join("Library/Application Support/Code/User/settings.json");
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
/// if let Some(path) = get_vscode_keybindings_path() {
///     println!("VSCode keybindings at {:?}", path);
/// }
/// ```
pub fn get_vscode_keybindings_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let vscode_path = home_dir.join("Library/Application Support/Code/User/keybindings.json");
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
/// if let Some(path) = get_ghostty_config_path() {
///     println!("Ghostty config at {:?}", path);
/// }
/// ```
pub fn get_ghostty_config_path() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let ghostty_path = home_dir.join("Library/Application Support/com.mitchellh.ghostty/config");
    if ghostty_path.exists() {
        Some(ghostty_path)
    } else {
        None
    }
}
