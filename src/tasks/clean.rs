use glob::glob;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::cli::CleanArgs;
use crate::logger::log;
use crate::utils::filesystem::calculate_dir_size;
use crate::utils::format::bytes_to_human_readable;
use crate::utils::paths::get_base_dirs;
use crate::utils::system::run_cmd;

/// Performs a system junk cleanup by deleting cache, logs, trash, and other temporary files.
///
/// The cleanup is divided into two categories:
/// - User-level cleanup: Files owned by the current user (default behavior)
/// - System-level cleanup: System-wide files requiring sudo (with --system flag)
///
/// The function is cross-platform aware and only applies platform-specific
/// cleanup operations when running on the appropriate OS.
pub fn run(args: CleanArgs) {
    log(&format!("Starting clean (system: {})", args.system));
    println!("🧹 Cleaning system junk...");

    let mut total_space_saved: u64 = 0;

    // Always clean user-level directories
    total_space_saved += clean_user_directories();

    // Clean system-level directories only if requested
    if args.system {
        println!("⚠️  Cleaning system-wide files (requires sudo)...");
        total_space_saved += clean_system_directories();
    }

    // Platform-specific cleanup
    #[cfg(target_os = "macos")]
    {
        clean_macos_specific(&args);
    }

    // Cross-platform package manager cleanup
    total_space_saved += clean_package_managers();

    // Clean trash for current user
    // TODO: Implement cross-platform trash cleaning

    let space_saved_str = bytes_to_human_readable(total_space_saved);
    println!("✅ System cleaned. Freed {}.", space_saved_str);
    log(&format!("Clean complete. Freed {}.", space_saved_str));
}

/// Clean user-level directories that don't require sudo
fn clean_user_directories() -> u64 {
    println!("🔹 Cleaning user directories...");

    let mut total_freed = 0u64;
    let mut user_paths = Vec::new();

    // Get base directories
    let base_dirs = get_base_dirs();
    let home_dir = base_dirs.home_dir();
    let cache_dir = base_dirs.cache_dir();

    // Cross-platform user cache directory
    user_paths.push(cache_dir.to_path_buf());

    // Cross-platform user data directory
    user_paths.push(std::env::temp_dir());

    // Platform-specific user directories
    #[cfg(target_os = "macos")]
    {
        user_paths.extend([
            home_dir.join("Library/Logs"),
            home_dir.join("Library/Saved Application State"),
        ]);
    }

    for path in user_paths {
        total_freed += clean_directory_contents(&path, false);
    }

    total_freed
}

/// Clean system-level directories that require sudo
fn clean_system_directories() -> u64 {
    let mut total_freed = 0u64;

    let mut system_paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        system_paths.extend([
            PathBuf::from("/Library/Caches"),
            PathBuf::from("/private/var/log"),
            PathBuf::from("/Library/Logs/DiagnosticReports"),
        ]);

        // Volume trashes
        if let Ok(entries) = glob("/Volumes/*/.Trashes") {
            for entry in entries.filter_map(Result::ok) {
                system_paths.push(entry);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        system_paths.extend([
            PathBuf::from("/var/log"),
            PathBuf::from("/var/cache"),
            PathBuf::from("/tmp"),
        ]);
    }

    for path in system_paths {
        total_freed += clean_directory_contents(&path, true);
    }

    total_freed
}

/// macOS-specific cleanup operations
#[cfg(target_os = "macos")]
fn clean_macos_specific(_args: &CleanArgs) -> () {
    // Reset Quick Look cache (user-level)
    println!("🔹 Resetting Quick Look cache...");
    let _ = run_cmd("qlmanage", &["-r", "cache"]);
}

/// Clean package manager caches
fn clean_package_managers() -> u64 {
    let total_freed = 0u64;

    // Homebrew cleanup (macOS/Linux)
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        if which::which("brew").is_ok() {
            println!("🍺 Running brew cleanup...");
            let _ = run_cmd("brew", &["cleanup"]);
        }
    }

    // npm cache cleanup (cross-platform)
    if which::which("npm").is_ok() {
        println!("📦 Cleaning npm cache...");
        let _ = run_cmd("npm", &["cache", "clean", "--force"]);
    }

    // pnpm cache cleanup (cross-platform)
    if which::which("pnpm").is_ok() {
        println!("📦 Cleaning pnpm cache...");
        let _ = run_cmd("pnpm", &["cache", "delete"]);
    }

    total_freed
}

/// Clean contents of a directory
fn clean_directory_contents(dir_path: &Path, use_sudo: bool) -> u64 {
    if !dir_path.exists() {
        return 0;
    }

    let mut total_freed = 0u64;
    let now = SystemTime::now();
    let min_age = Duration::from_secs(24 * 60 * 60); // 24 hours

    let glob_pattern = format!("{}/*", dir_path.display());
    let entries = match glob(&glob_pattern) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    for entry in entries.filter_map(Result::ok) {
        if !entry.exists() {
            continue;
        }

        // Skip symlinks to avoid accidental deletions
        if let Ok(metadata) = entry.symlink_metadata() {
            if metadata.file_type().is_symlink() {
                continue;
            }

            // Skip files modified within the last 24 hours
            if let Ok(modified) = metadata.modified() {
                if now.duration_since(modified).unwrap_or_default() < min_age {
                    continue;
                }
            }
        }

        let space = calculate_dir_size(&entry).unwrap_or(0);
        total_freed += space;

        // Remove the file/directory
        if use_sudo {
            // Try fs operations first, fall back to sudo if permission denied
            let result = if entry.is_dir() {
                fs::remove_dir_all(&entry)
            } else {
                fs::remove_file(&entry)
            };

            if result.is_err() {
                let _ = run_cmd("sudo", &["rm", "-rf", entry.to_str().unwrap()]);
            }
        } else {
            let _ = fs::remove_dir_all(&entry).or_else(|_| fs::remove_file(&entry));
        }
    }

    total_freed
}
