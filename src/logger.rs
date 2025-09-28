use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

use crate::utils::paths::get_mntn_dir;

/// Appends a timestamped log message to a file named `mntn.log` in the `.mntn` directory.
///
/// The log entry format is: `[YYYY-MM-DD HH:MM:SS] message`
pub fn log(message: &str) {
    let mntn_dir = get_mntn_dir();
    let log_path = mntn_dir.join("mntn.log"); // ~/.mntn/mntn.log
    let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap();

    writeln!(file, "{} {}", timestamp, message).unwrap();
}
