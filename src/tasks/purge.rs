use crate::cli::PurgeArgs;
use crate::logger::log;
use crate::utils::paths::get_base_dirs;
use crate::utils::system::run_cmd;
use inquire::MultiSelect;
use std::fs;
use std::path::PathBuf;

/// Represents a directory target for scanning service files
#[derive(Debug, Clone)]
struct DirectoryTarget {
    name: &'static str,
    path: &'static str,
    is_system: bool,
}

/// Represents a found service/autostart file with metadata
#[derive(Debug)]
struct ServiceFile {
    display_label: String,
    path: PathBuf,
    is_system: bool,
    service_type: ServiceType,
}

/// Types of services that can be managed
#[derive(Debug, Clone)]
enum ServiceType {
    #[cfg(target_os = "macos")]
    Plist,
    #[cfg(target_os = "linux")]
    SystemdService,
    #[cfg(target_os = "linux")]
    AutostartDesktop,
    #[cfg(target_os = "windows")]
    WindowsService,
    #[cfg(target_os = "windows")]
    StartupProgram,
}

/// Lists all system services and startup programs from standard directories,
/// prompts the user to select which ones to delete, and deletes the selected files.
pub fn run(args: PurgeArgs) {
    #[cfg(target_os = "macos")]
    {
        println!("🧼 Listing all launch agents and daemons...");
        log("Starting plist listing");
    }
    #[cfg(target_os = "linux")]
    {
        println!("🧼 Listing all systemd services and autostart programs...");
        log("Starting systemd service listing");
    }
    #[cfg(target_os = "windows")]
    {
        println!("🧼 Listing all Windows services and startup programs...");
        log("Starting Windows service listing");
    }

    let targets = get_directory_targets(args.system);
    let service_files = scan_service_files(&targets);

    if service_files.is_empty() {
        #[cfg(target_os = "macos")]
        println!("📁 No .plist files found.");
        #[cfg(target_os = "linux")]
        println!("📁 No systemd services or autostart programs found.");
        #[cfg(target_os = "windows")]
        println!("📁 No Windows services or startup programs found.");
        log("No service files found.");
        return;
    }

    // Collect display labels for user selection prompt
    let options: Vec<String> = service_files
        .iter()
        .map(|f| f.display_label.clone())
        .collect();

    // Prompt user to select files to delete (multi-select)
    let action_verb = if args.dry_run {
        "preview deletion for"
    } else {
        "delete"
    };
    let service_type_name = get_service_type_name();
    let prompt_message = format!("Select {} to {}:", service_type_name, action_verb);

    let to_delete = MultiSelect::new(&prompt_message, options.clone())
        .prompt()
        .unwrap_or_default();

    if args.dry_run {
        println!("🔍 Dry run - would delete the following items:");
        for selected in to_delete {
            if let Some(service_file) = service_files.iter().find(|f| f.display_label == selected) {
                println!("  - {}", service_file.path.display());
                log(&format!("Would delete: {}", service_file.path.display()));
            }
        }
        println!("✅ Dry run complete. No files were actually deleted.");
        log("Dry run complete");
    } else {
        for selected in to_delete {
            if let Some(service_file) = service_files.iter().find(|f| f.display_label == selected) {
                delete_service_file(service_file);
                log(&format!("Deleted: {}", service_file.path.display()));
            }
        }
        log("Service deletion complete");
        println!("✅ Selected items deleted.");
    }
}

/// Returns the directory targets to scan based on the system flag and platform
fn get_directory_targets(include_system: bool) -> Vec<DirectoryTarget> {
    let mut targets = Vec::new();

    let base_dirs = get_base_dirs();
    let home_dir = base_dirs.home_dir();
    #[cfg(target_os = "linux")]
    let config_dir = base_dirs.config_dir();

    #[cfg(target_os = "macos")]
    {
        targets.push(DirectoryTarget {
            name: "User LaunchAgents",
            path: home_dir.join("Library/LaunchAgents").to_str().unwrap_or(""),
            is_system: false,
        });

        if include_system {
            targets.push(DirectoryTarget {
                name: "System LaunchAgents",
                path: "/Library/LaunchAgents",
                is_system: true,
            });
            targets.push(DirectoryTarget {
                name: "System LaunchDaemons",
                path: "/Library/LaunchDaemons",
                is_system: true,
            });
        }
    }

    #[cfg(target_os = "linux")]
    {
        targets.push(DirectoryTarget {
            name: "User Systemd Services",
            path: config_dir.join("systemd/user").to_str().unwrap_or(""),
            is_system: false,
        });
        targets.push(DirectoryTarget {
            name: "User Autostart",
            path: config_dir.join("autostart").to_str().unwrap_or(""),
            is_system: false,
        });

        if include_system {
            targets.push(DirectoryTarget {
                name: "System Systemd Services",
                path: "/etc/systemd/system",
                is_system: true,
            });
            targets.push(DirectoryTarget {
                name: "System Systemd Services (lib)",
                path: "/lib/systemd/system",
                is_system: true,
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        // TODO: Windows implementation to be added
        println!("⚠️  Windows support is not yet implemented for the purge command.");
    }

    targets
}

/// Scans the specified directories for service files and returns them with metadata
fn scan_service_files(targets: &[DirectoryTarget]) -> Vec<ServiceFile> {
    let mut service_files = Vec::new();

    for target in targets {
        let path = PathBuf::from(target.path);
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                let service_path = entry.path();

                let (service_type, should_include) = determine_service_type(&service_path, target);
                if !should_include {
                    continue;
                }

                let display_label =
                    get_service_display_label(target.name, &service_path, &service_type);

                service_files.push(ServiceFile {
                    display_label,
                    path: service_path,
                    is_system: target.is_system,
                    service_type,
                });
            }
        }
    }

    service_files
}

/// Gets the service type name for the current platform
fn get_service_type_name() -> &'static str {
    #[cfg(target_os = "macos")]
    return "plist files";
    #[cfg(target_os = "linux")]
    return "services and autostart programs";
    #[cfg(target_os = "windows")]
    return "services and startup programs";
}

/// Determines the service type and whether the file should be included
fn determine_service_type(service_path: &PathBuf, target: &DirectoryTarget) -> (ServiceType, bool) {
    let extension = service_path.extension().and_then(|s| s.to_str());

    #[cfg(target_os = "macos")]
    {
        if extension == Some("plist") {
            return (ServiceType::Plist, true);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if extension == Some("service") || extension == Some("timer") || extension == Some("socket")
        {
            return (ServiceType::SystemdService, true);
        }
        if extension == Some("desktop") && target.path.contains("autostart") {
            return (ServiceType::AutostartDesktop, true);
        }
    }

    #[cfg(target_os = "windows")]
    {
        // TODO: Implement Windows service detection
        let _ = target;
        return (ServiceType::WindowsService, false);
    }

    #[allow(unreachable_code)]
    {
        let _ = target;
        (ServiceType::Plist, false)
    }
}

/// Gets a friendly display label for a service file
fn get_service_display_label(
    group_name: &str,
    service_path: &PathBuf,
    service_type: &ServiceType,
) -> String {
    match service_type {
        #[cfg(target_os = "macos")]
        ServiceType::Plist => {
            let label_result = run_cmd(
                "defaults",
                &["read", service_path.to_str().unwrap_or(""), "Label"],
            );

            let label = match label_result {
                Ok(output) => output.trim().to_string(),
                Err(_) => String::new(),
            };

            if !label.is_empty() {
                format!("[{}] {}", group_name, label)
            } else {
                let fallback = service_path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("unknown.plist");
                format!("[{}] {}", group_name, fallback)
            }
        }
        #[cfg(target_os = "linux")]
        ServiceType::SystemdService => {
            let service_name = service_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unknown.service");

            let description = get_systemd_service_description(service_path)
                .unwrap_or_else(|| service_name.to_string());

            format!("[{}] {} ({})", group_name, service_name, description)
        }
        #[cfg(target_os = "linux")]
        ServiceType::AutostartDesktop => {
            let app_name = service_path
                .file_name()
                .and_then(|f| f.to_str())
                .and_then(|s| s.strip_suffix(".desktop"))
                .unwrap_or("unknown");

            let display_name =
                get_desktop_file_name(service_path).unwrap_or_else(|| app_name.to_string());

            format!("[{}] {}", group_name, display_name)
        }
        #[cfg(target_os = "windows")]
        ServiceType::WindowsService | ServiceType::StartupProgram => {
            let fallback = service_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unknown");
            format!("[{}] {}", group_name, fallback)
        }
    }
}

#[cfg(target_os = "linux")]
/// Gets the description from a systemd service file
fn get_systemd_service_description(service_path: &PathBuf) -> Option<String> {
    let content = fs::read_to_string(service_path).ok()?;

    for line in content.lines() {
        if line.starts_with("Description=") {
            return Some(line.strip_prefix("Description=")?.trim().to_string());
        }
    }

    None
}

#[cfg(target_os = "linux")]
/// Gets the name from a desktop file
fn get_desktop_file_name(desktop_path: &PathBuf) -> Option<String> {
    let content = fs::read_to_string(desktop_path).ok()?;

    for line in content.lines() {
        if line.starts_with("Name=") {
            return Some(line.strip_prefix("Name=")?.trim().to_string());
        }
    }

    None
}

/// Attempts to delete a service file, with platform-specific handling
fn delete_service_file(service_file: &ServiceFile) {
    match &service_file.service_type {
        #[cfg(target_os = "macos")]
        ServiceType::Plist => {
            delete_file_with_sudo(&service_file.path, service_file.is_system);
        }
        #[cfg(target_os = "linux")]
        ServiceType::SystemdService => {
            // For systemd services, first try to stop and disable the service
            if let Some(service_name) = service_file.path.file_name().and_then(|f| f.to_str()) {
                let systemctl_cmd = if service_file.is_system {
                    "sudo systemctl"
                } else {
                    "systemctl --user"
                };

                // Try to stop the service (ignore errors as it might not be running)
                let _ = if service_file.is_system {
                    run_cmd("sudo", &["systemctl", "stop", service_name])
                } else {
                    run_cmd("systemctl", &["--user", "stop", service_name])
                };

                // Try to disable the service (ignore errors as it might not be enabled)
                let _ = if service_file.is_system {
                    run_cmd("sudo", &["systemctl", "disable", service_name])
                } else {
                    run_cmd("systemctl", &["--user", "disable", service_name])
                };

                println!("🚫 Stopped and disabled service: {}", service_name);
            }

            delete_file_with_sudo(&service_file.path, service_file.is_system);
        }
        #[cfg(target_os = "linux")]
        ServiceType::AutostartDesktop => {
            delete_file_with_sudo(&service_file.path, service_file.is_system);
        }
        #[cfg(target_os = "windows")]
        ServiceType::WindowsService | ServiceType::StartupProgram => {
            // TODO: Implement Windows service deletion
            println!("⚠️  Windows service deletion not yet implemented");
        }
    }
}

/// Helper function to delete a file, using sudo if needed for system files
fn delete_file_with_sudo(path: &PathBuf, is_system_file: bool) {
    match fs::remove_file(path) {
        Ok(_) => {
            println!("🗑️  Deleted: {}", path.display());
        }
        Err(_) => {
            if is_system_file {
                println!("🔐 Requires elevated privileges, using sudo...");
                let result = std::process::Command::new("sudo")
                    .arg("rm")
                    .arg("-f")
                    .arg(path)
                    .status();

                match result {
                    Ok(status) if status.success() => {
                        println!("🗑️  Deleted with sudo: {}", path.display());
                    }
                    _ => {
                        println!("❌ Failed to delete: {}", path.display());
                        log(&format!("Failed to delete: {}", path.display()));
                    }
                }
            } else {
                println!("❌ Failed to delete: {}", path.display());
                log(&format!("Failed to delete: {}", path.display()));
            }
        }
    }
}
