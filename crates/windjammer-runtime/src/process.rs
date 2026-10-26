//! Process management
//!
//! Windjammer's `std::process` module maps to these functions.

use std::process::Command;

/// Run a command and return output
pub fn run(program: &str, args: &[String]) -> Result<String, String> {
    crate::caps::process(program);
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| e.to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Run a command and return full output
pub fn run_with_output(program: &str, args: &[String]) -> Result<ProcessOutput, String> {
    crate::caps::process(program);
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    Ok(ProcessOutput {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Exit the current process
pub fn exit(code: i32) -> ! {
    std::process::exit(code);
}

/// Current process id (`int` in Windjammer).
pub fn pid() -> i64 {
    std::process::id() as i64
}

/// Process output
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_is_this_process() {
        assert_eq!(pid(), std::process::id() as i64);
        assert!(pid() > 0);
    }

    #[test]
    fn test_run_echo() {
        #[cfg(unix)]
        {
            let result = run("echo", &["hello".to_string()]);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().trim(), "hello");
        }
    }

    #[test]
    fn test_run_with_output() {
        #[cfg(unix)]
        {
            let result = run_with_output("echo", &["test".to_string()]);
            assert!(result.is_ok());
            let output = result.unwrap();
            assert_eq!(output.status, 0);
            assert_eq!(output.stdout.trim(), "test");
        }
    }
}
