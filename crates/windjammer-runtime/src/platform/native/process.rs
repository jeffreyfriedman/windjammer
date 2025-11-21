//! Native process implementation
//!
//! Re-exports the existing windjammer-runtime process module.

// Re-export all functions from the parent process module
pub use crate::process::*;

/// Command output structure (compatible with WASM version)
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

// Add any additional functions needed by std::process API
pub fn execute(command: String, args: Vec<String>) -> Result<CommandOutput, String> {
    use std::process::Command;

    let output = Command::new(&command)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    Ok(CommandOutput {
        stdout,
        stderr,
        status,
    })
}

pub fn spawn(command: String, args: Vec<String>) -> Result<ProcessHandle, String> {
    use std::process::Command;

    let child = Command::new(&command)
        .args(&args)
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    Ok(ProcessHandle {
        id: child.id() as i32,
    })
}

#[derive(Debug, Clone)]
pub struct ProcessHandle {
    pub id: i32,
}
