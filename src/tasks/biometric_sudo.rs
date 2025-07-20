use crate::logger::log;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

const SUDO_PAM_PATH: &str = "/etc/pam.d/sudo";
const TOUCH_ID_LINE: &str = "auth       sufficient     pam_tid.so\n";
const BACKUP_SUFFIX: &str = ".backup";

/// Entry point to configure Touch ID for `sudo` authentication.
///
/// Prints status messages and logs the process.
///
/// If configuration succeeds, informs the user; if it fails, reports the error.
///
/// # Panics
///
/// This function itself does not panic; errors from `configure_biometric_sudo` are handled gracefully.
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
///
/// # Returns
///
/// Returns `Ok(())` on success, or an `io::Error` if any IO or command operation fails.
///
/// # Errors
///
/// Returns errors from:
/// - Creating temporary file.
/// - Executing `sudo cat` or `sudo cp` commands.
/// - Reading or writing files.
///
/// # Important
///
/// This function must be run with sufficient privileges (sudo rights) to modify system files.
///
/// # Notes
///
/// The PAM file modification is done by prepending the Touch ID authentication line,
/// which may have implications if other PAM rules rely on ordering.
fn configure_biometric_sudo() -> io::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path();

    // Read current sudo PAM file content
    let content = Command::new("sudo")
        .arg("cat")
        .arg(SUDO_PAM_PATH)
        .output()?;

    let content = String::from_utf8_lossy(&content.stdout);

    // Check if Touch ID is already configured
    if content.contains(TOUCH_ID_LINE.trim()) {
        println!("ℹ️ Touch ID authentication is already configured");
        return Ok(());
    }

    // Prepend Touch ID PAM line to existing content
    let new_content = format!("{}{}", TOUCH_ID_LINE, content);
    fs::write(temp_path, new_content)?;

    // Create backup if it does not already exist
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

    // Overwrite sudo PAM file with new content
    Command::new("sudo")
        .arg("cp")
        .arg(temp_path)
        .arg(SUDO_PAM_PATH)
        .status()?;

    Ok(())
}
