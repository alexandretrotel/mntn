use crate::cli::InstallArgs;
use crate::logger::log;
use crate::utils::paths::get_base_dirs;
use std::fs;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::process::Command;
use which::which;

/// Installs macOS LaunchAgents to automate recurring maintenance tasks.
///
/// This function creates `.plist` files in `~/Library/LaunchAgents` for:
/// - `com.mntn.backup` → runs `mntn backup` every hour
/// - `com.mntn.clean` → runs `mntn clean` every day
/// - `com.mntn.topgrade` → runs `topgrade` every day (only if installed)
///
/// After writing each agent's configuration, it loads the agent with `launchctl`.
pub fn run(args: InstallArgs) {
    println!("📦 Installing scheduled tasks...");
    log("Starting scheduled task installation");

    let mut tasks: Vec<ScheduledTask> = vec![ScheduledTask::backup_hourly()];
    if which("topgrade").is_ok() {
        tasks.push(ScheduledTask::topgrade_daily());
    } else {
        println!("⚠️ topgrade not found, skipping daily topgrade task.");
        log("topgrade not found, skipping daily topgrade task");
    }
    if args.with_clean {
        tasks.push(ScheduledTask::clean_daily());
    }

    for task in tasks.into_iter() {
        if let Err(e) = task.install() {
            println!("❌ Failed to install {}: {}", task.label(), e);
            log(&format!("Failed to install {}: {}", task.label(), e));
        }
    }

    println!("✅ Scheduled tasks installed.");
    log("Scheduled tasks installed");
}

/// A struct representing a scheduled task configuration.
enum ScheduledTaskKind {
    MacLaunchd,
    SystemdUser,
    WindowsTask,
}

struct ScheduledTask {
    label: String,
    binary: String,
    args: Vec<String>,
    /// Interval in seconds (macOS / Linux). Windows uses a translated schedule.
    interval: u32,
}

impl ScheduledTask {
    fn backup_hourly() -> Self {
        Self::new("mntn-backup", "mntn", &["backup"], 3600)
    }
    fn clean_daily() -> Self {
        Self::new("mntn-clean", "mntn", &["clean"], 86400)
    }
    fn topgrade_daily() -> Self {
        Self::new("mntn-topgrade", "topgrade", &[], 86400)
    }
    fn new(label: &str, binary: &str, args: &[&str], interval: u32) -> Self {
        Self {
            label: label.into(),
            binary: binary.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            interval,
        }
    }
    fn label(&self) -> &str {
        &self.label
    }

    fn install(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        {
            return self.install_launchd();
        }
        #[cfg(target_os = "linux")]
        {
            return self.install_systemd_user();
        }
        #[cfg(target_os = "windows")]
        {
            return self.install_windows();
        }
        #[allow(unreachable_code)]
        Err("Unsupported platform".into())
    }

    #[cfg(target_os = "macos")]
    fn install_launchd(&self) -> Result<(), Box<dyn std::error::Error>> {
        let binary_path = which(&self.binary)?.to_str().unwrap().to_string();
        let base_dirs = get_base_dirs();
        let home_dir = base_dirs.home_dir();
        let plist_path = home_dir
            .join("Library/LaunchAgents")
            .join(format!("{}.plist", self.label));
        if let Some(dir) = plist_path.parent() {
            fs::create_dir_all(dir)?;
        }
        let args_xml = self
            .args
            .iter()
            .map(|a| format!("    <string>{}</string>", a))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>{label}</string>
  <key>ProgramArguments</key><array>
    <string>{binary}</string>
    {args}
  </array>
  <key>StartInterval</key><integer>{interval}</integer>
  <key>RunAtLoad</key><true/>
  <key>StandardOutPath</key><string>/tmp/{label}.out</string>
  <key>StandardErrorPath</key><string>/tmp/{label}.err</string>
</dict></plist>
"#,
            label = self.label,
            binary = binary_path,
            args = args_xml,
            interval = self.interval
        );
        fs::write(&plist_path, content)?;
        Command::new("launchctl")
            .arg("load")
            .arg(&plist_path)
            .output()?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn install_systemd_user(&self) -> Result<(), Box<dyn std::error::Error>> {
        let binary_path = which(&self.binary)?.to_str().unwrap().to_string();
        let base_dirs = get_base_dirs();
        let config_dir = base_dirs.config_dir();
        fs::create_dir_all(&config_dir)?;
        let service_name = format!("{}.service", self.label);
        let timer_name = format!("{}.timer", self.label);
        let service_path = config_dir.join(&service_name);
        let timer_path = config_dir.join(&timer_name);
        let exec = format!("{} {}", binary_path, self.args.join(" "));
        let service_content = format!(
            "[Unit]\nDescription=Run {} task\n\n[Service]\nType=oneshot\nExecStart={}\n",
            self.label, exec
        );
        let timer_content = if self.interval % 3600 == 0 {
            let hours = self.interval / 3600;
            if hours == 1 {
                "[Unit]\nDescription=Hourly task\n\n[Timer]\nOnCalendar=hourly\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n".to_string()
            } else if hours == 24 {
                "[Unit]\nDescription=Daily task\n\n[Timer]\nOnCalendar=daily\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n".to_string()
            } else {
                format!(
                    "[Unit]\nDescription=Every {} hours task\n\n[Timer]\nOnUnitActiveSec={}h\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n",
                    hours, hours
                )
            }
        } else {
            format!(
                "[Unit]\nDescription=Interval task {}s\n\n[Timer]\nOnUnitActiveSec={}s\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n",
                self.interval, self.interval
            )
        };
        fs::write(&service_path, service_content)?;
        fs::write(&timer_path, timer_content)?;

        Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output()?;
        Command::new("systemctl")
            .args(["--user", "enable", &timer_name])
            .output()?;
        Command::new("systemctl")
            .args(["--user", "start", &timer_name])
            .output()?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn install_windows(&self) -> Result<(), Box<dyn std::error::Error>> {
        let binary_path = which(&self.binary)?.to_str().unwrap().to_string();
        let task_name = format!("mntn-{}", self.label);
        let mut schedule = String::from("HOURLY");
        if self.interval >= 23 * 3600 {
            schedule = "DAILY".into();
        }
        let exec = format!("\"{}\" {}", binary_path, self.args.join(" "));
        Command::new("schtasks")
            .args([
                "/Create", "/SC", &schedule, "/TN", &task_name, "/TR", &exec, "/F",
            ])
            .output()?;
        Ok(())
    }
}
