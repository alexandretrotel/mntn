use crate::logger::log;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

const SUDO_PAM_PATH: &str = "/etc/pam.d/sudo";
const TOUCH_ID_LINE: &str = "auth       sufficient     pam_tid.so\n";
const BACKUP_SUFFIX: &str = ".backup";

pub fn run() {
    println!("🔐 Configuring Touch ID for sudo...");
    log("Starting Touch ID sudo configuration");

    match configure_biometric_sudo() {
        Ok(_) => {
            println!("✅ Touch ID authentication successfully configured for sudo");
            log("Touch ID authentication configured successfully");
        }
        Err(e) => {
            println!("❌ Failed to configure Touch ID authentication: {}", e);
            log(&format!("Failed to configure Touch ID: {}", e));
        }
    }
}

fn configure_biometric_sudo() -> io::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path();

    let content = Command::new("sudo")
        .arg("cat")
        .arg(SUDO_PAM_PATH)
        .output()?;

    let content = String::from_utf8_lossy(&content.stdout);

    if content.contains(TOUCH_ID_LINE.trim()) {
        println!("ℹ️ Touch ID authentication is already configured");
        return Ok(());
    }

    let new_content = format!("{}{}", TOUCH_ID_LINE, content);
    fs::write(temp_path, new_content)?;

    let backup_path = format!("{}{}", SUDO_PAM_PATH, BACKUP_SUFFIX);
    if !Path::new(&backup_path).exists() {
        Command::new("sudo")
            .arg("cp")
            .arg(SUDO_PAM_PATH)
            .arg(&backup_path)
            .status()?;
        println!("📦 Created backup at {}", backup_path);
        log(&format!("Created backup at {}", backup_path));
    } else {
        println!("ℹ️ Backup file already exists at {}", backup_path);
    }

    Command::new("sudo")
        .arg("cp")
        .arg(temp_path)
        .arg(SUDO_PAM_PATH)
        .status()?;

    Ok(())
}
