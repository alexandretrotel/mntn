use std::io;
use std::{fs, path::Path, process::Command};

pub fn run_cmd(cmd: &str, args: &[&str]) -> String {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .expect("Failed to run command");

    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn calculate_dir_size(path: &Path) -> io::Result<u64> {
    let mut total_size: u64 = 0;

    for entry in glob::glob(path.to_str().unwrap()).unwrap().flatten() {
        let metadata = fs::metadata(&entry)?;
        if metadata.is_file() {
            total_size += metadata.len();
        } else if metadata.is_dir() {
            total_size += calculate_dir_size(&entry)?;
        }
    }

    Ok(total_size)
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
