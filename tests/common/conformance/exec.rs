//! Process execution with a wall-clock budget (std only).

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Exec {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
    pub timed_out: bool,
    pub elapsed: Duration,
}

fn drain<R: Read + Send + 'static>(mut r: R) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = r.read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).to_string()
    })
}

pub fn run_with_timeout(cmd: &mut Command, budget: Duration) -> Exec {
    let start = Instant::now();
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Exec { stdout: String::new(), stderr: format!("spawn failed: {e}"), code: -1, timed_out: false, elapsed: start.elapsed() }
        }
    };
    let out = drain(child.stdout.take().unwrap());
    let err = drain(child.stderr.take().unwrap());
    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) if start.elapsed() > budget => {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break -1;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => break -1,
        }
    };
    Exec { stdout: out.join().unwrap_or_default(), stderr: err.join().unwrap_or_default(), code, timed_out, elapsed: start.elapsed() }
}
