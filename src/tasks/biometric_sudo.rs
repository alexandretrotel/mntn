use tempfile::NamedTempFile;

use crate::logger::log;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const TOUCH_ID_LINE: &str = "auth       sufficient     pam_tid.so\n";
const BACKUP_SUFFIX: &str = ".bak";
const SUDO_PAM_PATH: &str = "/etc/pam.d/sudo";

/// If configuration succeeds, informs the user; if it fails, reports the error.
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

/// Configures the sudo PAM file to enable Touch ID authentication.
///
/// Steps performed:
/// - Reads current `/etc/pam.d/sudo` file using `sudo cat`.
/// - Checks if Touch ID PAM line is already present.
/// - If missing, prepends the PAM Touch ID line to the file contents.
/// - Creates a backup of the original PAM file as `/etc/pam.d/sudo.backup` if not existing.
/// - Overwrites the original PAM file with the modified content via `sudo cp`.
fn configure_biometric_sudo() -> io::Result<()> {
    let sudo_path = Path::new(SUDO_PAM_PATH);

    // Check if Touch ID is already configured
    let content = fs::read_to_string(sudo_path)?;
    if content
        .lines()
        .any(|line| line.trim() == TOUCH_ID_LINE.trim())
    {
        println!("ℹ️ Touch ID authentication is already configured");
        return Ok(());
    }

    // Backup if not already there
    let backup_path = sudo_path.with_file_name({
        let mut backup_name = OsString::from(sudo_path.file_name().unwrap());
        backup_name.push(BACKUP_SUFFIX);
        backup_name
    });
    if !Path::new(&backup_path).exists() {
        fs::copy(SUDO_PAM_PATH, &backup_path)?;
        println!("📦 Created backup at {}", backup_path.display());
    } else {
        println!(
            "ℹ️ Backup already exists at {}, skipping backup",
            backup_path.display()
        );
    }

    // Prepend Touch ID line safely
    let mut new_content = String::with_capacity(content.len() + TOUCH_ID_LINE.len());
    new_content.push_str(TOUCH_ID_LINE);
    new_content.push_str(&content);

    // Write atomically to a temporary file
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(new_content.as_bytes())?;

    // Persist the temporary file to the target path
    temp_file.persist(sudo_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to persist updated PAM file: {}", e),
        )
    })?;

    Ok(())
}
