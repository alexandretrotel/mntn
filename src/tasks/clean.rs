use crate::logger::log;
use crate::utils::run_cmd;
use shellexpand::tilde;

pub fn run() {
    log("Starting clean");
    println!("🧹 Cleaning system junk...");

    let dirs = vec![
        "~/Library/Caches/*",
        "/Library/Caches/*",
        "/private/var/log/*",
        "~/Library/Logs/*",
        "~/Library/Saved Application State/*",
        "~/Library/Logs/DiagnosticReports/*",
        "/Library/Logs/DiagnosticReports/*",
        "~/.Trash/*",
        "/private/var/root/.Trash/*",
        "/Volumes/*/.Trashes",
    ];

    for dir in dirs {
        let expanded = tilde(dir).to_string();
        let _ = run_cmd("rm", &["-rf", &expanded]);
    }

    let _ = run_cmd("qlmanage", &["-r", "cache"]);
    println!("✅ System cleaned.");
    log("Clean complete");
}
