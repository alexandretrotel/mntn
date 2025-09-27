use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

/// Appends a timestamped log message to a file named `mntn.log` in the user's home directory.
///
/// The log entry format is: `[YYYY-MM-DD HH:MM:SS] message`
///
/// # Panics
///
/// This function will panic if:
/// - The user's home directory cannot be determined.
/// - The log file cannot be created or opened for appending.
/// - Writing to the log file fails.
///
/// # Examples
///
/// ```
/// log("Application started");
/// ```
pub fn log(message: &str) {
    let log_path = dirs_next::home_dir().unwrap().join("mntn.log"); // ~/mntn.log
    let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap();

    writeln!(file, "{} {}", timestamp, message).unwrap();
}
