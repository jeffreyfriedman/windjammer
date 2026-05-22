//! Interactive subprocess I/O (spawn + piped stdin/stdout).
//!
//! Windjammer `std::subprocess` maps here. Uses opaque handles so backends can swap
//! implementation without changing the WJ API.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use once_cell::sync::Lazy;

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
static SESSIONS: Lazy<Mutex<HashMap<u64, SubprocessInner>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct SubprocessInner {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// Opaque handle returned to Windjammer (`Subprocess.handle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubprocessHandle {
    pub id: u64,
}

impl SubprocessHandle {
    pub fn invalid() -> Self {
        Self { id: 0 }
    }

    pub fn is_valid(self) -> bool {
        self.id != 0
    }
}

pub fn invalid_handle() -> SubprocessHandle {
    SubprocessHandle::invalid()
}

pub fn is_valid(handle: SubprocessHandle) -> bool {
    handle.is_valid()
}

/// Spawn a program with piped stdin/stdout (stderr inherits to host).
pub fn spawn(program: &str, args: &[String]) -> Result<SubprocessHandle, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("spawn {}: {}", program, e))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "stdin unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "stdout unavailable".to_string())?;

    let id = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    SESSIONS.lock().unwrap().insert(
        id,
        SubprocessInner {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        },
    );
    Ok(SubprocessHandle { id })
}

fn with_session<F, T>(handle: SubprocessHandle, f: F) -> Result<T, String>
where
    F: FnOnce(&mut SubprocessInner) -> Result<T, String>,
{
    if !handle.is_valid() {
        return Err("invalid subprocess handle".to_string());
    }
    let mut guard = SESSIONS.lock().unwrap();
    let session = guard
        .get_mut(&handle.id)
        .ok_or_else(|| "subprocess not found".to_string())?;
    f(session)
}

pub fn write_line(handle: SubprocessHandle, line: &str) -> Result<(), String> {
    with_session(handle, |s| {
        writeln!(s.stdin, "{}", line).map_err(|e| e.to_string())?;
        s.stdin.flush().map_err(|e| e.to_string())
    })
}

pub fn read_line(handle: SubprocessHandle) -> Result<String, String> {
    with_session(handle, |s| {
        let mut buf = String::new();
        s.stdout
            .read_line(&mut buf)
            .map_err(|e| e.to_string())?;
        if buf.is_empty() {
            return Err("stdout closed".to_string());
        }
        Ok(buf.trim_end().to_string())
    })
}

/// Read lines until one starts with `prefix` (e.g. `@AGENT OBS`).
pub fn read_line_until_prefix(handle: SubprocessHandle, prefix: &str) -> Result<String, String> {
    loop {
        let line = read_line(handle)?;
        if line.starts_with(prefix) {
            return Ok(line);
        }
    }
}

pub fn kill(handle: SubprocessHandle) -> Result<(), String> {
    with_session(handle, |s| s.child.kill().map_err(|e| e.to_string()))
}

pub fn wait(handle: SubprocessHandle) -> Result<i32, String> {
    with_session(handle, |s| {
        s.child
            .wait()
            .map_err(|e| e.to_string())
            .map(|st| st.code().unwrap_or(-1))
    })
}

pub fn close(handle: SubprocessHandle) -> Result<(), String> {
    if !handle.is_valid() {
        return Ok(());
    }
    let mut guard = SESSIONS.lock().unwrap();
    guard.remove(&handle.id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_echo_line() {
        #[cfg(unix)]
        {
            let h = spawn("echo", &["hello".to_string()]).unwrap();
            let line = read_line(h).unwrap();
            assert_eq!(line.trim(), "hello");
            let _ = wait(h);
            let _ = close(h);
        }
    }

    #[test]
    fn spawn_cat_write_read() {
        #[cfg(unix)]
        {
            let h = spawn("cat", &[]).unwrap();
            write_line(h, "ping").unwrap();
            let line = read_line(h).unwrap();
            assert_eq!(line, "ping");
            let _ = kill(h);
            let _ = close(h);
        }
    }
}
