use std::io;
use std::path::Path;
use std::process::Command;

/// Runs a system command with the given arguments and returns its standard output as a `String`.
///
/// - Captures stdout and returns it if the command succeeds and output is valid UTF-8.
/// - Returns an `io::Error` if the command cannot be run.
/// - Returns an error if the command exits with non-zero status.
/// - Returns a `FromUtf8Error` if stdout isn't valid UTF-8.
pub fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    run_cmd_impl(cmd, args, None)
}

/// Run a command in a specific directory
pub fn run_cmd_in_dir(
    cmd: &str,
    args: &[&str],
    dir: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    run_cmd_impl(cmd, args, Some(dir))
}

/// Internal implementation for running commands with optional directory
fn run_cmd_impl(
    cmd: &str,
    args: &[&str],
    dir: Option<&Path>,
) -> Result<String, Box<dyn std::error::Error>> {
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

        return Err(io::Error::other(format!(
            "Command '{}' failed with status {:?}: {}",
            cmd,
            output.status.code(),
            stderr_msg
        ))
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout)
}

/// Synchronizes a directory using rsync with delete option
///
/// This copies the contents of source to destination, deleting any files
/// in destination that don't exist in source.
pub fn rsync_directory(source: &Path, dest: &Path) -> io::Result<()> {
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
