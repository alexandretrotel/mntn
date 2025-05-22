use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

pub fn log(message: &str) {
    let log_path = dirs::home_dir().unwrap().join("mntn.log"); // ~/mntn.log
    let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap();

    writeln!(file, "{} {}", timestamp, message).unwrap();
}
