use crate::logger::log;
use crate::tasks::{backup, clean};
use std::fs;
use std::process::Command;
use which::which;

pub fn run() {
    let bin_path = which("mntn").expect("mntn binary not found in PATH");
    println!("📦 Using binary: {}", bin_path.display());

    let agents = vec![
        ("com.mntn.backup", vec!["backup"], 3600),
        ("com.mntn.clean", vec!["clean"], 86400),
    ];

    for (label, args, interval) in agents {
        let plist_path = dirs::home_dir()
            .unwrap()
            .join("Library/LaunchAgents")
            .join(format!("{}.plist", label));

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>{}</string>
  </array>
  <key>StartInterval</key>
  <integer>{}</integer>
  <key>RunAtLoad</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/{}.out</string>
  <key>StandardErrorPath</key>
  <string>/tmp/{}.err</string>
</dict>
</plist>"#,
            label,
            bin_path.display(),
            args[0],
            interval,
            label,
            label
        );

        fs::create_dir_all(plist_path.parent().unwrap()).unwrap();
        fs::write(&plist_path, content).unwrap();
        let _ = Command::new("launchctl")
            .arg("load")
            .arg(plist_path)
            .output();
    }

    println!("✅ Launch agents installed and loaded.");
    log("Installed and loaded launch agents.");

    backup::run();
    clean::run();
}
