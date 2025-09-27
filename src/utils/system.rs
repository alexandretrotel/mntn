use std::process::Command;

/// Runs a system command with the given arguments and returns its standard output as a `String`.
///
/// If the command fails to run, returns an empty string.
///
/// # Arguments
///
/// * `cmd` - The command to run (e.g., "ls", "echo").
/// * `args` - A slice of argument strings to pass to the command.
///
/// # Examples
///
/// ```
/// use mntn::utils::system::run_cmd;
///
/// let output = run_cmd("echo", &["hello"]);
/// assert_eq!(output.trim(), "hello");
/// ```
pub fn run_cmd(cmd: &str, args: &[&str]) -> String {
    let output = Command::new(cmd).args(args).output();

    match output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(_) => String::new(),
    }
}
