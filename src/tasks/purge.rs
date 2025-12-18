use crate::cli::PurgeArgs;
use crate::logger::{log, log_error, log_info, log_success};
use crate::tasks::core::{PlannedOperation, Task, TaskExecutor};
use crate::utils::paths::get_base_dirs;
#[cfg(not(windows))]
use crate::utils::system::run_cmd;
use inquire::MultiSelect;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;

/// Represents a directory target for scanning service files
#[derive(Debug, Clone)]
struct DirectoryTarget {
    name: &'static str,
    path: PathBuf,
    is_system: bool,
}

/// Represents a found service/autostart file with metadata
#[derive(Debug)]
struct ServiceFile {
    display_label: String,
    path: PathBuf,
    is_system: bool,
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

/// Purge task that removes system services and startup programs
pub struct PurgeTask {
    pub system: bool,
}

impl PurgeTask {
    pub fn new(system: bool) -> Self {
        Self { system }
    }
}

impl Task for PurgeTask {
    fn name(&self) -> &str {
        "Purge"
    }

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        println!("🧼 Listing all launch agents and daemons...");
        #[cfg(target_os = "linux")]
        println!("🧼 Listing all systemd services and autostart programs...");
        #[cfg(target_os = "windows")]
        println!("🧼 Listing all Windows services and startup programs...");

        let targets = get_directory_targets(self.system);
        let service_files = scan_service_files(&targets);

        if service_files.is_empty() {
            #[cfg(target_os = "macos")]
            log_info("No .plist files found");
            #[cfg(target_os = "linux")]
            log_info("No systemd services or autostart programs found");
            #[cfg(target_os = "windows")]
            log_info("No Windows services or startup programs found");
            return Ok(());
        }

        let options: Vec<String> = service_files
            .iter()
            .map(|f| f.display_label.clone())
            .collect();

        let service_type_name = get_service_type_name();
        let prompt_message = format!("Select {} to delete:", service_type_name);

        let to_delete = match MultiSelect::new(&prompt_message, options.clone()).prompt() {
            Ok(selected) => selected,
            Err(_) => {
                log_info("Selection cancelled");
                return Ok(());
            }
        };

        if to_delete.is_empty() {
            log_info("No items selected");
            return Ok(());
        }

        for selected in to_delete {
            if let Some(service_file) = service_files.iter().find(|f| f.display_label == selected) {
                delete_service_file(service_file);
                log(&format!("Deleted: {}", service_file.path.display()));
            }
        }

        log_success("Selected items deleted");

        Ok(())
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let targets = get_directory_targets(self.system);
        let service_files = scan_service_files(&targets);

        for service_file in service_files {
            operations.push(PlannedOperation::with_target(
                format!("Would scan: {}", service_file.display_label),
                service_file.path.display().to_string(),
            ));
        }

        if operations.is_empty() {
            operations.push(PlannedOperation::new("No service files found to purge"));
        }

        operations
    }
}

/// Run with CLI args
pub fn run_with_args(args: PurgeArgs) {
    let mut task = PurgeTask::new(args.system);
    TaskExecutor::run(&mut task, args.dry_run);
}

/// Returns the directory targets to scan based on the system flag and platform
fn get_directory_targets(include_system: bool) -> Vec<DirectoryTarget> {
    let mut targets = Vec::new();

    let base_dirs = get_base_dirs();

    #[cfg(target_os = "macos")]
    let home_dir = base_dirs.home_dir();
    #[cfg(target_os = "linux")]
    let config_dir = base_dirs.config_dir();

    #[cfg(target_os = "macos")]
    {
        targets.push(DirectoryTarget {
            name: "User LaunchAgents",
            path: home_dir.join("Library/LaunchAgents"),
            is_system: false,
        });

        if include_system {
            targets.push(DirectoryTarget {
                name: "System LaunchAgents",
                path: PathBuf::from("/Library/LaunchAgents"),
                is_system: true,
            });
            targets.push(DirectoryTarget {
                name: "System LaunchDaemons",
                path: PathBuf::from("/Library/LaunchDaemons"),
                is_system: true,
            });
        }
    }

    #[cfg(target_os = "linux")]
    {
        targets.push(DirectoryTarget {
            name: "User Systemd Services",
            path: config_dir.join("systemd/user"),
            is_system: false,
        });
        targets.push(DirectoryTarget {
            name: "User Autostart",
            path: config_dir.join("autostart"),
            is_system: false,
        });

        if include_system {
            targets.push(DirectoryTarget {
                name: "System Systemd Services",
                path: PathBuf::from("/etc/systemd/system"),
                is_system: true,
            });
            targets.push(DirectoryTarget {
                name: "System Systemd Services (lib)",
                path: PathBuf::from("/lib/systemd/system"),
                is_system: true,
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        // No actual directories needed; handled in scan_service_files
    }

    targets
}

/// Scans the specified directories for service files and returns them with metadata
fn scan_service_files(targets: &[DirectoryTarget]) -> Vec<ServiceFile> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let mut service_files = Vec::new();

        for target in targets {
            let path = &target.path;
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let service_path = entry.path();

                    let (service_type, should_include) =
                        determine_service_type(&service_path, target);
                    if !should_include {
                        continue;
                    }

                    let display_label =
                        get_service_display_label(target.name, &service_path, &service_type);

                    service_files.push(ServiceFile {
                        display_label,
                        path: service_path,
                        is_system: target.is_system,
                    });
                }
            }
        }

        service_files
    }

    #[cfg(target_os = "windows")]
    {
        let mut services = list_windows_services();
        let mut startups = list_startup_programs();
        services.append(&mut startups);
        services
    }
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
fn determine_service_type(service_path: &Path, target: &DirectoryTarget) -> (ServiceType, bool) {
    let extension = service_path.extension().and_then(|s| s.to_str());
    let _ = target;

    #[cfg(target_os = "macos")]
    {
        let should_include = extension == Some("plist");
        (ServiceType::Plist, should_include)
    }

    #[cfg(target_os = "linux")]
    {
        if extension == Some("service") || extension == Some("timer") || extension == Some("socket")
        {
            return (ServiceType::SystemdService, true);
        }
        if extension == Some("desktop")
            && target
                .path
                .components()
                .any(|c| c.as_os_str() == "autostart")
        {
            return (ServiceType::AutostartDesktop, true);
        }
        return (ServiceType::SystemdService, false);
    }

    #[cfg(target_os = "windows")]
    {
        (ServiceType::WindowsService, false)
    }
}

/// Gets a friendly display label for a service file
fn get_service_display_label(
    group_name: &str,
    service_path: &Path,
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
fn get_systemd_service_description(service_path: &Path) -> Option<String> {
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
fn get_desktop_file_name(desktop_path: &Path) -> Option<String> {
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
    #[cfg(target_os = "macos")]
    {
        delete_file_with_sudo(&service_file.path, service_file.is_system);
    }

    #[cfg(target_os = "linux")]
    match &service_file.service_type {
        ServiceType::SystemdService => {
            // For systemd services, first try to stop and disable the service
            if let Some(service_name) = service_file.path.file_name().and_then(|f| f.to_str()) {
                let _ = if service_file.is_system {
                    run_cmd("sudo", &["systemctl", "stop", service_name])
                } else {
                    run_cmd("systemctl", &["--user", "stop", service_name])
                };
                let _ = if service_file.is_system {
                    run_cmd("sudo", &["systemctl", "disable", service_name])
                } else {
                    run_cmd("systemctl", &["--user", "disable", service_name])
                };
                log_info(&format!("Stopped and disabled service: {}", service_name));
            }
            delete_file_with_sudo(&service_file.path, service_file.is_system);
        }
        ServiceType::AutostartDesktop => {
            delete_file_with_sudo(&service_file.path, service_file.is_system);
        }
    }

    #[cfg(target_os = "windows")]
    match service_file.service_type {
        ServiceType::StartupProgram => {
            if let Err(e) = fs::remove_file(&service_file.path) {
                log_error(
                    &format!(
                        "Failed to delete startup program: {}",
                        service_file.path.display()
                    ),
                    e,
                );
            } else {
                log_success(&format!(
                    "Deleted startup program: {}",
                    service_file.path.display()
                ));
            }
        }
        ServiceType::WindowsService => {
            if let Some(service_name) = service_file.path.file_name().and_then(|f| f.to_str()) {
                let _ = Command::new("sc").args(&["stop", service_name]).status();
                let _ = Command::new("sc").args(&["delete", service_name]).status();
                log_success(&format!("Deleted Windows service: {}", service_name));
            }
        }
    }
}

/// Helper function to delete a file, using sudo if needed for system files
fn delete_file_with_sudo(path: &PathBuf, is_system_file: bool) {
    match fs::remove_file(path) {
        Ok(_) => {
            log_success(&format!("Deleted: {}", path.display()));
        }
        Err(_) => {
            if is_system_file {
                log_info("Requires elevated privileges, using sudo...");
                let result = std::process::Command::new("sudo")
                    .arg("rm")
                    .arg("-f")
                    .arg(path)
                    .status();

                match result {
                    Ok(status) if status.success() => {
                        log_success(&format!("Deleted with sudo: {}", path.display()));
                    }
                    _ => {
                        log_error("Failed to delete", path.display());
                    }
                }
            } else {
                log_error("Failed to delete", path.display());
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn list_windows_services() -> Vec<ServiceFile> {
    let output = match Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-Command",
            "Get-Service | Select-Object Name, Status | ConvertTo-Json -Compress",
        ])
        .output()
    {
        Ok(output) => output,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();

    // Parse JSON output - can be an array or single object
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        let items = match &json {
            serde_json::Value::Array(arr) => arr.as_slice(),
            obj @ serde_json::Value::Object(_) => std::slice::from_ref(obj),
            _ => return services,
        };

        for item in items {
            if let Some(name) = item.get("Name").and_then(|v| v.as_str()) {
                let status = item
                    .get("Status")
                    .and_then(|v| v.as_u64())
                    .map(|s| match s {
                        1 => "Stopped",
                        4 => "Running",
                        _ => "Unknown",
                    })
                    .unwrap_or("Unknown");

                services.push(ServiceFile {
                    display_label: format!("[Windows Service] {} ({})", name, status),
                    path: PathBuf::from(name),
                    is_system: false,
                });
            }
        }
    }

    services
}

#[cfg(target_os = "windows")]
fn list_startup_programs() -> Vec<ServiceFile> {
    use std::env;

    let mut startup_files = Vec::new();

    let user_startup = env::var("APPDATA")
        .map(|appdata| PathBuf::from(appdata).join("Microsoft/Windows/Start Menu/Programs/Startup"))
        .ok();

    let all_users_startup = env::var("PROGRAMDATA")
        .map(|programdata| {
            PathBuf::from(programdata).join("Microsoft/Windows/Start Menu/Programs/Startup")
        })
        .ok();

    for dir in [user_startup, all_users_startup].iter().flatten() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    startup_files.push(ServiceFile {
                        display_label: format!(
                            "[Startup Program] {}",
                            path.file_name().unwrap().to_string_lossy()
                        ),
                        path: path.clone(),
                        is_system: false,
                        service_type: ServiceType::StartupProgram,
                    });
                }
            }
        }
    }

    startup_files
}
