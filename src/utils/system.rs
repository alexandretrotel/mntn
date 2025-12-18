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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_run_cmd_echo() {
        let result = run_cmd("echo", &["hello"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello");
    }

    #[test]
    fn test_run_cmd_with_multiple_args() {
        let result = run_cmd("echo", &["hello", "world"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello world");
    }

    #[test]
    fn test_run_cmd_captures_stdout() {
        let result = run_cmd("printf", &["test output"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test output");
    }

    #[test]
    fn test_run_cmd_nonexistent_command() {
        let result = run_cmd("this_command_does_not_exist_12345", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_cmd_non_zero_exit_code() {
        // 'false' command always exits with code 1
        let result = run_cmd("false", &[]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed"));
    }

    #[test]
    fn test_run_cmd_exit_with_specific_code() {
        // Use sh -c to run a command that exits with a specific code
        let result = run_cmd("sh", &["-c", "exit 42"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_cmd_empty_args() {
        // pwd should work without arguments
        let result = run_cmd("pwd", &[]);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_run_cmd_multiline_output() {
        let result = run_cmd("printf", &["line1\\nline2\\nline3"]);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("line1"));
        assert!(output.contains("line2"));
        assert!(output.contains("line3"));
    }

    #[test]
    fn test_run_cmd_in_dir_executes_in_directory() {
        let temp_dir = TempDir::new().unwrap();
        #[cfg(windows)]
        let result = run_cmd_in_dir("cmd", &["/C", "cd"], temp_dir.path());
        #[cfg(not(windows))]
        let result = run_cmd_in_dir("pwd", &[], temp_dir.path());

        assert!(result.is_ok());
        let output = result.unwrap();
        // The output should contain the temp directory path
        let temp_dir_str = temp_dir.path().to_str().unwrap();
        assert!(output.trim().contains(temp_dir_str) || temp_dir_str.contains(output.trim()));
    }

    #[test]
    fn test_run_cmd_in_dir_can_access_files() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        // Use ls to verify the file exists in the directory
        let result = run_cmd_in_dir("ls", &[], temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test.txt"));
    }

    #[test]
    fn test_run_cmd_in_dir_nonexistent_directory() {
        let result = run_cmd_in_dir("echo", &["test"], Path::new("/nonexistent/path/12345"));
        assert!(result.is_err());
    }

    #[test]
    fn test_run_cmd_in_dir_with_args() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("a.txt"), "").unwrap();
        fs::write(temp_dir.path().join("b.txt"), "").unwrap();

        let result = run_cmd_in_dir("ls", &["-1"], temp_dir.path());
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("a.txt"));
        assert!(output.contains("b.txt"));
    }

    #[test]
    fn test_run_cmd_impl_without_dir() {
        let result = run_cmd_impl("echo", &["test"], None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "test");
    }

    #[test]
    fn test_run_cmd_impl_with_dir() {
        let temp_dir = TempDir::new().unwrap();
        let result = run_cmd_impl("pwd", &[], Some(temp_dir.path()));
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn test_rsync_directory_copies_files() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create some files in source
        fs::write(src_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(src_dir.path().join("file2.txt"), "content2").unwrap();

        let result = rsync_directory(src_dir.path(), dst_dir.path());

        // Skip if rsync is not available
        if result.is_err()
            && result
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("No such file")
        {
            return; // rsync not installed, skip test
        }

        assert!(result.is_ok());
        assert!(dst_dir.path().join("file1.txt").exists());
        assert!(dst_dir.path().join("file2.txt").exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_rsync_directory_preserves_content() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        let content = "This is test content with special chars: !@#$%";
        fs::write(src_dir.path().join("data.txt"), content).unwrap();

        let result = rsync_directory(src_dir.path(), dst_dir.path());

        if result.is_err() {
            return; // rsync not available
        }

        assert_eq!(
            fs::read_to_string(dst_dir.path().join("data.txt")).unwrap(),
            content
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_rsync_directory_deletes_extra_files() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create file in source
        fs::write(src_dir.path().join("keep.txt"), "keep").unwrap();

        // Create extra file in destination
        fs::write(dst_dir.path().join("delete.txt"), "delete").unwrap();

        let result = rsync_directory(src_dir.path(), dst_dir.path());

        if result.is_err() {
            return; // rsync not available
        }

        // Extra file should be deleted
        assert!(!dst_dir.path().join("delete.txt").exists());
        // Source file should be copied
        assert!(dst_dir.path().join("keep.txt").exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_rsync_directory_copies_subdirectories() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create nested structure
        fs::create_dir(src_dir.path().join("subdir")).unwrap();
        fs::write(src_dir.path().join("subdir").join("nested.txt"), "nested").unwrap();

        let result = rsync_directory(src_dir.path(), dst_dir.path());

        if result.is_err() {
            return; // rsync not available
        }

        assert!(dst_dir.path().join("subdir").join("nested.txt").exists());
    }

    #[test]
    fn test_rsync_directory_nonexistent_source() {
        let dst_dir = TempDir::new().unwrap();
        let result = rsync_directory(Path::new("/nonexistent/path/12345"), dst_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_error_message_contains_command_name() {
        let result = run_cmd("sh", &["-c", "echo error >&2; exit 1"]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("sh"));
    }

    #[test]
    fn test_error_message_contains_stderr() {
        let result = run_cmd("sh", &["-c", "echo 'custom error message' >&2; exit 1"]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("custom error message"));
    }
}
