//! Child-process helper with a wall-clock timeout. Output is captured, never echoed: compiler
//! messages can quote source text, and the reducer must not print input source.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct Captured {
    pub code: Option<i32>,
    /// Killed by a signal (segfault, abort, ...).
    pub signaled: bool,
    pub timed_out: bool,
    pub stdout: String,
    pub stderr: String,
}

impl Captured {
    pub fn success(&self) -> bool {
        self.code == Some(0) && !self.timed_out
    }
}

pub fn run(mut cmd: Command, timeout: Duration) -> Captured {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => {
            return Captured {
                code: Some(127),
                ..Default::default()
            }
        }
    };
    let mut out = child.stdout.take().expect("piped");
    let mut err = child.stderr.take().expect("piped");
    let t_out = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        String::from_utf8_lossy(&b).into_owned()
    });
    let t_err = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        String::from_utf8_lossy(&b).into_owned()
    });
    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() > timeout => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok();
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => break None,
        }
    };
    let stdout = t_out.join().unwrap_or_default();
    let stderr = t_err.join().unwrap_or_default();
    Captured {
        code: status.and_then(|s| s.code()),
        signaled: status.is_some_and(|s| s.code().is_none()) && !timed_out,
        timed_out,
        stdout,
        stderr,
    }
}
