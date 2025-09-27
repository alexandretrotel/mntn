use crate::logger::log;
use std::fs;
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
///
/// # Notes
/// - Skips agents if their binary is not found via `which`.
/// - All `.plist` logs go to `/tmp/{label}.out/.err`.
/// - Gracefully logs failures and proceeds with available tasks.
///
/// # Example
/// ```
/// install_launch_agents::run();
/// ```
pub fn run() {
    println!("📦 Installing launch agents...");
    log("Starting launch agent installation");

    let mut agents: Vec<LaunchAgent> = vec![
        LaunchAgent::new("com.mntn.backup", "mntn", &["backup"], 3600),
        LaunchAgent::new("com.mntn.clean", "mntn", &["clean"], 86400),
    ];

    // Only add topgrade if it's installed
    if which("topgrade").is_ok() {
        agents.push(LaunchAgent::new(
            "com.mntn.topgrade",
            "topgrade",
            &[],
            86400,
        ));
    } else {
        println!("⚠️ topgrade not found, skipping launch agent installation.");
        log("topgrade not found, skipping launch agent installation");
    }

    for agent in agents {
        if let Err(e) = agent.install() {
            println!("❌ Failed to install {}: {}", agent.label, e);
            log(&format!("Failed to install {}: {}", agent.label, e));
        }
    }

    println!("✅ Launch agents installed and loaded.");
    log("Launch agents installed and loaded");
}

/// A struct representing a macOS LaunchAgent configuration.
struct LaunchAgent {
    label: String,
    binary_name: String,
    args: Vec<String>,
    interval: u32,
}

impl LaunchAgent {
    fn new(label: &str, binary_name: &str, args: &[&str], interval: u32) -> Self {
        LaunchAgent {
            label: label.to_string(),
            binary_name: binary_name.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            interval,
        }
    }

    /// Writes the `.plist` file and loads it via `launchctl`.
    fn install(&self) -> Result<(), Box<dyn std::error::Error>> {
        let binary_path = which(&self.binary_name)?.to_str().unwrap().to_string();

        let plist_path = dirs_next::home_dir()
            .ok_or("Failed to determine home directory")?
            .join("Library/LaunchAgents")
            .join(format!("{}.plist", self.label));

        let plist_dir = plist_path.parent().ok_or("Invalid plist path")?;
        fs::create_dir_all(plist_dir)?;

        let args_xml = self
            .args
            .iter()
            .map(|arg| format!("<string>{}</string>", arg))
            .collect::<Vec<_>>()
            .join("\n    ");

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{binary}</string>
    {args}
  </array>
  <key>StartInterval</key>
  <integer>{interval}</integer>
  <key>RunAtLoad</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/{label}.out</string>
  <key>StandardErrorPath</key>
  <string>/tmp/{label}.err</string>
</dict>
</plist>
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
}
