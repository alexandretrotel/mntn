use std::process::Command;

pub fn run_cmd(cmd: &str, args: &[&str]) -> String {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .expect("Failed to run command");

    String::from_utf8_lossy(&output.stdout).to_string()
}
