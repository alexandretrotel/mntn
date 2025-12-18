use glob::glob;
use std::fs;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::cli::CleanArgs;
use crate::tasks::core::{PlannedOperation, Task, TaskExecutor};
use crate::utils::filesystem::calculate_dir_size;
use crate::utils::format::bytes_to_human_readable;
use crate::utils::paths::get_base_dirs;
use crate::utils::system::run_cmd;

/// Clean task that removes cache, logs, trash, and other temporary files
pub struct CleanTask {
    pub system: bool,
}

impl CleanTask {
    pub fn new(system: bool) -> Self {
        Self { system }
    }
}

impl Task for CleanTask {
    fn name(&self) -> &str {
        "Clean"
    }

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🧹 Cleaning system junk...");

        let args = CleanArgs {
            system: self.system,
            dry_run: false,
        };

        let mut total_space_saved: u64 = 0;

        total_space_saved += clean_user_directories(&args);

        if self.system {
            println!("⚠️  Cleaning system-wide files (requires sudo)...");
            total_space_saved += clean_system_directories(&args);
        }

        #[cfg(target_os = "macos")]
        {
            clean_macos_specific(&args);
        }

        total_space_saved += clean_package_managers(&args);

        total_space_saved += clean_trash();

        let space_saved_str = bytes_to_human_readable(total_space_saved);
        println!("✅ System cleaned. Freed {}.", space_saved_str);

        Ok(())
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let base_dirs = get_base_dirs();
        let cache_dir = base_dirs.cache_dir();

        #[cfg(target_os = "macos")]
        let home_dir = base_dirs.home_dir();

        // User directories
        operations.push(PlannedOperation::with_target(
            "Clean user cache".to_string(),
            cache_dir.display().to_string(),
        ));
        operations.push(PlannedOperation::with_target(
            "Clean temp directory".to_string(),
            std::env::temp_dir().display().to_string(),
        ));

        #[cfg(target_os = "macos")]
        {
            operations.push(PlannedOperation::with_target(
                "Clean user logs".to_string(),
                home_dir.join("Library/Logs").display().to_string(),
            ));
            operations.push(PlannedOperation::with_target(
                "Clean saved application state".to_string(),
                home_dir
                    .join("Library/Saved Application State")
                    .display()
                    .to_string(),
            ));
        }

        if self.system {
            #[cfg(target_os = "macos")]
            {
                operations.push(PlannedOperation::with_target(
                    "Clean system caches".to_string(),
                    "/Library/Caches".to_string(),
                ));
                operations.push(PlannedOperation::with_target(
                    "Clean system logs".to_string(),
                    "/private/var/log".to_string(),
                ));
            }
            #[cfg(target_os = "linux")]
            {
                operations.push(PlannedOperation::with_target(
                    "Clean system logs".to_string(),
                    "/var/log".to_string(),
                ));
                operations.push(PlannedOperation::with_target(
                    "Clean system cache".to_string(),
                    "/var/cache".to_string(),
                ));
            }
        }

        // Package managers
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        if which::which("brew").is_ok() {
            operations.push(PlannedOperation::new("Run brew cleanup"));
        }
        if which::which("npm").is_ok() {
            operations.push(PlannedOperation::new("Clean npm cache"));
        }
        if which::which("pnpm").is_ok() {
            operations.push(PlannedOperation::new("Clean pnpm cache"));
        }

        // Trash
        operations.push(PlannedOperation::new("Empty trash"));

        operations
    }
}

/// Run with CLI args
pub fn run_with_args(args: CleanArgs) {
    let mut task = CleanTask::new(args.system);
    TaskExecutor::run(&mut task, args.dry_run);
}

/// Clean user-level directories that don't require sudo
fn clean_user_directories(args: &CleanArgs) -> u64 {
    println!("🔹 Cleaning user directories...");

    let mut total_freed = 0u64;
    let mut user_paths = Vec::new();

    // Get base directories
    let base_dirs = get_base_dirs();
    let cache_dir = base_dirs.cache_dir();

    #[cfg(target_os = "macos")]
    let home_dir = base_dirs.home_dir();

    // Cross-platform user cache directory
    user_paths.push(cache_dir.to_path_buf());

    // Cross-platform user temp directory
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
        total_freed += clean_directory_contents(&path, false, args);
    }

    total_freed
}

/// Clean system-level directories that require sudo
fn clean_system_directories(args: &CleanArgs) -> u64 {
    let mut total_freed = 0u64;

    let mut system_paths: Vec<PathBuf> = Vec::new();

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
        total_freed += clean_directory_contents(&path, true, args);
    }

    total_freed
}

/// macOS-specific cleanup operations
#[cfg(target_os = "macos")]
fn clean_macos_specific(args: &CleanArgs) {
    // Reset Quick Look cache (user-level)
    println!("🔹 Resetting Quick Look cache...");
    if !args.dry_run {
        let _ = run_cmd("qlmanage", &["-r", "cache"]);
    } else {
        println!("   [DRY RUN] Would reset Quick Look cache");
    }
}

/// Clean package manager caches
fn clean_package_managers(args: &CleanArgs) -> u64 {
    let total_freed = 0u64;

    // Homebrew cleanup (macOS/Linux)
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        if which::which("brew").is_ok() {
            println!("🍺 Running brew cleanup...");
            if !args.dry_run {
                let _ = run_cmd("brew", &["cleanup"]);
            } else {
                println!("   [DRY RUN] Would run brew cleanup");
            }
        }
    }

    // npm cache cleanup (cross-platform)
    if which::which("npm").is_ok() {
        println!("📦 Cleaning npm cache...");
        if !args.dry_run {
            let _ = run_cmd("npm", &["cache", "clean", "--force"]);
        } else {
            println!("   [DRY RUN] Would clean npm cache");
        }
    }

    // pnpm cache cleanup (cross-platform)
    if which::which("pnpm").is_ok() {
        println!("📦 Cleaning pnpm cache...");
        if !args.dry_run {
            let _ = run_cmd("pnpm", &["cache", "delete"]);
        } else {
            println!("   [DRY RUN] Would clean pnpm cache");
        }
    }

    total_freed
}

/// Clean contents of a directory
fn clean_directory_contents(dir_path: &Path, use_sudo: bool, args: &CleanArgs) -> u64 {
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

        // Skip if path matches skip patterns
        if should_skip(&entry) {
            continue;
        }

        // Skip symlinks to avoid accidental deletions
        if let Ok(metadata) = entry.symlink_metadata() {
            if metadata.file_type().is_symlink() {
                continue;
            }

            // Skip files modified within the last 24 hours
            if let Ok(modified) = metadata.modified()
                && now.duration_since(modified).unwrap_or_default() < min_age
            {
                continue;
            }
        }

        let space = calculate_dir_size(&entry).unwrap_or(0);
        total_freed += space;

        let space_str = bytes_to_human_readable(space);
        let entry_name = entry
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<invalid-utf8>");

        if args.dry_run {
            println!("   [DRY RUN] Would delete: {} ({})", entry_name, space_str);
            continue;
        }

        // Remove the file/directory
        if use_sudo {
            // Try fs operations first, fall back to sudo if permission denied
            let result = if entry.is_dir() {
                fs::remove_dir_all(&entry)
            } else {
                fs::remove_file(&entry)
            };

            if result.is_err() {
                if let Some(path_str) = entry.to_str() {
                    let _ = run_cmd("sudo", &["rm", "-rf", path_str]);
                } else {
                    println!("⚠️ Skipping non-UTF8 path: {:?}", entry);
                }
            }
        } else {
            let _ = fs::remove_dir_all(&entry).or_else(|_| fs::remove_file(&entry));
        }
    }

    total_freed
}

/// Check if a path should be skipped during cleanup
fn should_skip(path: &Path) -> bool {
    let skip_patterns = [".X11-unix", "systemd-private", "asl", ".DS_Store"];

    #[cfg(unix)]
    {
        skip_patterns.iter().any(|&pattern| {
            let pattern_bytes = pattern.as_bytes();

            path.file_name()
                .map(|name| {
                    name.as_bytes()
                        .windows(pattern_bytes.len())
                        .any(|window| window == pattern_bytes)
                })
                .unwrap_or(false)
                || path.components().any(|comp| {
                    comp.as_os_str()
                        .as_bytes()
                        .windows(pattern_bytes.len())
                        .any(|window| window == pattern_bytes)
                })
        })
    }

    #[cfg(not(unix))]
    {
        skip_patterns.iter().any(|&pattern| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.contains(pattern))
                .unwrap_or(false)
                || path.components().any(|comp| {
                    comp.as_os_str()
                        .to_str()
                        .is_some_and(|s| s.contains(pattern))
                })
        })
    }
}

/// Clean the trash/recycle bin for the current user
/// ⚠️ This ALWAYS executes — never a dry-run
fn clean_trash() -> u64 {
    let mut total_freed = 0u64;

    let base_dirs = get_base_dirs();
    let home_dir = base_dirs.home_dir();

    println!("🗑️  Emptying trash...");

    #[cfg(target_os = "macos")]
    {
        let trash_dir = home_dir.join(".Trash");
        total_freed += clean_directory_contents_force(&trash_dir);

        // External volume trash directories
        if let Ok(entries) = glob("/Volumes/*/.Trashes/*") {
            for entry in entries.filter_map(Result::ok) {
                total_freed += clean_directory_contents_force(&entry);
            }
        }

        if which::which("osascript").is_ok() {
            let script = r#"
                tell application "Finder"
                    empty trash
                end tell
            "#;
            let _ = run_cmd("osascript", &["-e", script]);
        }
    }

    #[cfg(target_os = "linux")]
    {
        total_freed += clean_directory_contents_force(
            &home_dir.join(".local/share/Trash/files"),
        );
        total_freed += clean_directory_contents_force(
            &home_dir.join(".local/share/Trash/info"),
        );
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let _ = Command::new("powershell")
            .args(&["-Command", "Clear-RecycleBin -Force"])
            .status();
    }

    total_freed
}

/// Force-delete directory contents
fn clean_directory_contents_force(dir: &Path) -> u64 {
    if !dir.exists() {
        return 0;
    }

    let mut freed = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            freed += calculate_dir_size(&path).unwrap_or(0);
            let _ = fs::remove_dir_all(&path).or_else(|_| fs::remove_file(&path));
        }
    }
    freed
}
