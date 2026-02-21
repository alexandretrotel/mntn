use crate::errors::{AppError, Result};
use regex::Regex;
use std::io;
use std::path::Path;
use std::process::Command;

pub fn strip_ansi_codes(input: &str) -> String {
    let re = Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").unwrap();
    re.replace_all(input, "").to_string()
}

pub fn run_cmd(cmd: &str, args: &[&str], dir: Option<&Path>) -> Result<String> {
    let mut command = Command::new(cmd);
    command.args(args);

    if let Some(d) = dir {
        command.current_dir(d);
    }

    let output = command.output()?;

    if !output.status.success() {
        let stderr_len = output.stderr.len();
        let stderr_msg = String::from_utf8(output.stderr)
            .unwrap_or_else(|_| format!("<non-UTF-8 stderr data: {} bytes>", stderr_len));

        return Err(AppError::CommandFailure {
            cmd: cmd.to_string(),
            status: output.status.code(),
            stderr: stderr_msg,
        });
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout)
}

pub fn sync_directory_contents(source: &Path, dest: &Path) -> io::Result<()> {
    let output = Command::new("rsync")
        .args(["-av", "--delete"])
        .arg(format!("{}/", source.display()))
        .arg(dest)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr.clone())
            .unwrap_or_else(|_| format!("<binary stderr: {} bytes>", output.stderr.len()));
        return Err(io::Error::other(format!("rsync failed: {}", stderr)));
    }

    Ok(())
}
