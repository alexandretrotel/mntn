use std::io;
use std::process::Command;

/// Runs a system command with the given arguments and returns its standard output as a `String`.
///
/// - Captures stdout and returns it if the command succeeds and output is valid UTF-8.
/// - Returns an `io::Error` if the command cannot be run.
/// - Returns an error if the command exits with non-zero status.
/// - Returns a `FromUtf8Error` if stdout isn’t valid UTF-8.
pub fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new(cmd).args(args).output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Command '{}' failed with status {:?}: {}",
                cmd,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ),
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout)
}
