//! The running crate host: spawned on the first crate call, kept for the run, spoken to over two pipes.
//!
//! Frames are `u32` little-endian length + body. The host's first frame is its cache key (a stale or foreign
//! binary is refused). A request is the function path (wire string) followed by the arguments; a reply is a
//! status byte: 0 = `u32` count + values (the result, then every `&mut` parameter), 1 = panic (message),
//! 2 = rejected request (message). On Unix the pipes are fds 3 and 4, so the crate's own stdout and stderr are
//! the program's, exactly as in the compiled build; elsewhere they are the host's stdin and stdout.

use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};

/// What a call came back with.
pub enum Reply {
    /// Encoded values: the result, then the `&mut` parameters in order.
    Values(Vec<u8>),
    Panic(String),
    Rejected(String),
}

/// Why no reply came.
pub enum Failure {
    /// The host could not be started (or announced another key).
    Start(String),
    /// The host process ended during the call.
    Died(Option<ExitStatus>),
}

struct Running {
    child: Child,
    tx: Box<dyn Write + Send>,
    rx: BufReader<Box<dyn Read + Send>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        // closing the request pipe ends the host's loop
        self.tx = Box::new(std::io::sink());
        let _ = self.child.wait();
    }
}

pub struct HostProcess {
    exe: PathBuf,
    key: String,
    running: Option<Running>,
    /// Calls made (for tests and `--result-json` style accounting).
    pub calls: u64,
}

fn read_frame(rx: &mut dyn Read) -> std::io::Result<Vec<u8>> {
    let mut lb = [0u8; 4];
    rx.read_exact(&mut lb)?;
    let mut body = vec![0u8; u32::from_le_bytes(lb) as usize];
    rx.read_exact(&mut body)?;
    Ok(body)
}

fn write_frame(tx: &mut dyn Write, body: &[u8]) -> std::io::Result<()> {
    let mut f = Vec::with_capacity(body.len() + 4);
    f.extend_from_slice(&(body.len() as u32).to_le_bytes());
    f.extend_from_slice(body);
    tx.write_all(&f)?;
    tx.flush()
}

#[cfg(unix)]
fn spawn(exe: &std::path::Path) -> std::io::Result<(Child, Box<dyn Write + Send>, Box<dyn Read + Send>)> {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;
    let (req_r, req_w) = std::io::pipe()?;
    let (rep_r, rep_w) = std::io::pipe()?;
    let (a, b) = (req_r.as_raw_fd(), rep_w.as_raw_fd());
    let mut cmd = Command::new(exe);
    cmd.env("WJ_CRATE_HOST_FDS", "1").stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit());
    // SAFETY: only async-signal-safe calls (fcntl, dup2) between fork and exec. Both ends are first moved above
    // 9 so placing one cannot overwrite the other; dup2 leaves 3 and 4 open across exec.
    unsafe {
        cmd.pre_exec(move || {
            let ha = libc::fcntl(a, libc::F_DUPFD, 10);
            let hb = libc::fcntl(b, libc::F_DUPFD, 10);
            if ha < 0 || hb < 0 || libc::dup2(ha, 3) < 0 || libc::dup2(hb, 4) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = cmd.spawn()?;
    drop(req_r);
    drop(rep_w);
    Ok((child, Box::new(req_w), Box::new(rep_r)))
}

#[cfg(not(unix))]
fn spawn(exe: &std::path::Path) -> std::io::Result<(Child, Box<dyn Write + Send>, Box<dyn Read + Send>)> {
    let mut child = Command::new(exe).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?;
    let tx = child.stdin.take().expect("piped stdin");
    let rx = child.stdout.take().expect("piped stdout");
    Ok((child, Box::new(tx), Box::new(rx)))
}

impl HostProcess {
    pub fn new(exe: PathBuf, key: String) -> HostProcess {
        HostProcess { exe, key, running: None, calls: 0 }
    }

    fn start(&mut self) -> Result<&mut Running, Failure> {
        if self.running.is_none() {
            let (child, tx, rx) = spawn(&self.exe).map_err(|e| Failure::Start(format!("cannot start the crate host {}: {e}", self.exe.display())))?;
            let mut r = Running { child, tx, rx: BufReader::new(rx) };
            let hello = read_frame(&mut r.rx).map_err(|_| Failure::Start(format!("the crate host {} exited before it started", self.exe.display())))?;
            if hello != self.key.as_bytes() {
                return Err(Failure::Start(format!(
                    "the crate host {} announced `{}`, expected `{}` (delete it to rebuild)",
                    self.exe.display(),
                    String::from_utf8_lossy(&hello),
                    self.key
                )));
            }
            self.running = Some(r);
        }
        Ok(self.running.as_mut().expect("started"))
    }

    /// One call. `request` is the encoded function path followed by the encoded arguments.
    pub fn call(&mut self, request: &[u8]) -> Result<Reply, Failure> {
        self.calls += 1;
        let r = self.start()?;
        let reply = write_frame(&mut r.tx, request).and_then(|_| read_frame(&mut r.rx));
        let body = match reply {
            Ok(b) => b,
            Err(_) => {
                // the host died: report how, and start a fresh one on the next call
                let mut dead = self.running.take().expect("running");
                dead.tx = Box::new(std::io::sink());
                let status = dead.child.wait().ok();
                return Err(Failure::Died(status));
            }
        };
        let text = |b: &[u8]| {
            let mut s = b;
            super::wire::decode(&mut s, &super::sig::WireTy::Str)
                .ok()
                .and_then(|v| match v {
                    super::super::value::Value::Str(s) => Some((*s).clone()),
                    _ => None,
                })
                .unwrap_or_default()
        };
        Ok(match body.first() {
            Some(0) => Reply::Values(body[1..].to_vec()),
            Some(1) => Reply::Panic(text(&body[1..])),
            _ => Reply::Rejected(text(body.get(1..).unwrap_or(&[]))),
        })
    }
}
