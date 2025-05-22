use crate::logger::log;
use crate::tasks::{backup, clean};
use std::fs;
use std::process::Command;
use which::which;

pub fn run() {
    let bin_path = which("mntn").expect("mntn binary not found in PATH");
    println!("📦 Using binary: {}", bin_path.display());

    let mut agents = vec![
        ("com.mntn.backup", vec!["backup"], 3600), // Hourly
        ("com.mntn.clean", vec!["clean"], 86400),  // Daily
    ];

    if let Ok(topgrade_path) = which("topgrade") {
        println!("📦 Found topgrade: {}", topgrade_path.display());
        agents.push(("com.mntn.topgrade", vec![], 86400)); // Daily
    } else {
        println!("⚠️ topgrade not found, skipping launch agent installation.");
        log("topgrade not found, skipping launch agent installation.");
    }

    for (label, args, interval) in agents {
        let plist_path = dirs::home_dir()
            .unwrap()
            .join("Library/LaunchAgents")
            .join(format!("{}.plist", label));

        let program = if label == "com.mntn.topgrade" {
            which("topgrade")
                .expect("topgrade binary not found")
                .to_str()
                .unwrap()
                .to_string()
        } else {
            bin_path.to_str().unwrap().to_string()
        };

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
    {}
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
            program,
            args.iter()
                .map(|arg| format!("<string>{}</string>", arg))
                .collect::<Vec<_>>()
                .join("\n    "),
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
