//! Shared plumbing for the agent-loop tools (`check_code`, `run_code`, `test_code`, `format_code`,
//! `docs_search`): argument validation, scratch files and locating the `wj` binary.

use crate::error::{McpError, McpResult};
use crate::protocol::ToolCallResult;
use crate::tools::text_response;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn parse_args<T: DeserializeOwned>(arguments: Value) -> McpResult<T> {
    serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
        field: "arguments".to_string(),
        message: e.to_string(),
    })
}

pub fn json_response(v: &Value) -> ToolCallResult {
    text_response(serde_json::to_string_pretty(v).unwrap_or_default())
}

/// A scratch directory removed on drop.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new() -> McpResult<Scratch> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("wj-mcp-{}-{}", std::process::id(), nanos));
        std::fs::create_dir_all(&dir).map_err(|e| McpError::InternalError {
            message: format!("cannot create scratch dir: {}", e),
        })?;
        Ok(Scratch(dir))
    }

    pub fn write(&self, name: &str, code: &str) -> McpResult<PathBuf> {
        let p = self.0.join(name);
        std::fs::write(&p, code).map_err(|e| McpError::InternalError {
            message: format!("cannot write {}: {}", p.display(), e),
        })?;
        Ok(p)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `WJ_BIN`, else a `wj` next to this executable, else `wj` on `PATH`.
pub fn wj_binary() -> PathBuf {
    if let Ok(p) = std::env::var("WJ_BIN") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join("wj");
            if cand.exists() {
                return cand;
            }
        }
    }
    PathBuf::from("wj")
}

pub struct Outcome {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

/// Run `wj <args>` in `cwd` with a wall-clock limit; the child is killed on timeout.
pub fn run_wj(args: &[&str], file: &Path, cwd: &Path, timeout: Duration) -> McpResult<Outcome> {
    let mut child = Command::new(wj_binary())
        .args(args)
        .arg(file)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| McpError::InternalError {
            message: format!("cannot start `{}` (set WJ_BIN): {}", wj_binary().display(), e),
        })?;
    let start = Instant::now();
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                timed_out = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break,
        }
    }
    let out = child.wait_with_output().map_err(|e| McpError::InternalError {
        message: e.to_string(),
    })?;
    Ok(Outcome {
        exit_code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        timed_out,
    })
}
