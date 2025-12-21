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

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) else {
        return;
    };

    let _ = writeln!(file, "{} {}", timestamp, message);
}

/// Logs and prints an error message
pub fn log_error(context: &str, error: impl std::fmt::Display) {
    let msg = format!("{}: {}", context, error);
    println!("❌ {}", msg);
    log(&msg);
}

/// Logs and prints a success message
pub fn log_success(message: &str) {
    println!("✅ {}", message);
    log(message);
}

/// Logs and prints a warning message
pub fn log_warning(message: &str) {
    println!("⚠️ {}", message);
    log(message);
}

/// Logs and prints an info message
pub fn log_info(message: &str) {
    println!("ℹ️ {}", message);
    log(message);
}
