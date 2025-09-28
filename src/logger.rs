use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

use crate::utils::paths::get_base_dirs;

/// Appends a timestamped log message to a file named `mntn.log` in the user's home directory.
///
/// The log entry format is: `[YYYY-MM-DD HH:MM:SS] message`
pub fn log(message: &str) {
    let base_dirs = get_base_dirs();
    let home_dir = base_dirs.home_dir();
    let log_path = home_dir.join("mntn.log"); // ~/mntn.log
    let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap();

    writeln!(file, "{} {}", timestamp, message).unwrap();
}
