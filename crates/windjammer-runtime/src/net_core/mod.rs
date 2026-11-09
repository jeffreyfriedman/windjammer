//! Raw sockets for `std::net` (ADR-0032): the one implementation behind `TcpStream`, `TcpListener` and `UdpSocket`.
//!
//! The compiled runtime (`platform::native::net`) wraps these types with its `caps::` checks; the interpreter includes
//! this file (`#[path]`) behind its capability gate and effect port. Buffering, timeouts, address resolution and the
//! error vocabulary live here only, so the two runtimes cannot drift apart. No capability checks happen here.

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, ToSocketAddrs};
use std::time::Duration;

/// The fixed error vocabulary programs match on (`Err("timed out")`, `Err("connection refused")`, ...).
pub fn describe(e: &io::Error) -> String {
    use io::ErrorKind::*;
    match e.kind() {
        TimedOut | WouldBlock => "timed out".into(),
        ConnectionRefused => "connection refused".into(),
        ConnectionReset | ConnectionAborted => "connection reset".into(),
        BrokenPipe => "broken pipe".into(),
        AddrInUse => "address in use".into(),
        AddrNotAvailable => "address not available".into(),
        UnexpectedEof => "unexpected end of stream".into(),
        NotConnected => "not connected".into(),
        _ => e.to_string(),
    }
}

fn err(e: io::Error) -> String {
    describe(&e)
}

/// A millisecond count as a socket timeout: zero or less means none.
pub fn timeout_of(ms: i64) -> Option<Duration> {
    (ms > 0).then(|| Duration::from_millis(ms as u64))
}

/// Resolve `host:port` exactly once. The capability check is made on the name as written; the connection then goes only
/// to these addresses, so the answer cannot change between the decision and the use (ADR-0032, address-verified connects).
pub fn resolve_once(addr: &str) -> Result<Vec<SocketAddr>, String> {
    let bad = || format!("invalid address: {addr}");
    let v: Vec<SocketAddr> = addr.to_socket_addrs().map_err(|_| bad())?.collect();
    if v.is_empty() {
        Err(bad())
    } else {
        Ok(v)
    }
}

/// Connect to the first of `addrs` that answers (the addresses `resolve_once` returned; never resolves again).
pub fn connect_resolved(addrs: &[SocketAddr], timeout: Option<Duration>) -> Result<Stream, String> {
    let mut last = io::Error::new(io::ErrorKind::AddrNotAvailable, "no address");
    for a in addrs {
        let r = match timeout {
            Some(t) => std::net::TcpStream::connect_timeout(a, t),
            None => std::net::TcpStream::connect(a),
        };
        match r {
            Ok(s) => return Ok(Stream { sock: s, buf: Vec::new() }),
            Err(e) => last = e,
        }
    }
    Err(err(last))
}

/// Shutting down a connection the peer has already closed is done, not an error (macOS reports `ENOTCONN` there).
fn quiet_shutdown(r: io::Result<()>) -> Result<(), String> {
    match r {
        Err(e) if e.kind() != io::ErrorKind::NotConnected => Err(err(e)),
        _ => Ok(()),
    }
}

fn size(n: i64) -> Result<usize, String> {
    if n < 1 {
        return Err(format!("read size must be at least 1, got {n}"));
    }
    Ok(n as usize)
}

/// A connected TCP stream with the read buffer `read_line` needs (and `read`/`read_exact` drain first).
pub struct Stream {
    sock: std::net::TcpStream,
    buf: Vec<u8>,
}

impl Stream {
    fn fill(&mut self, want: usize) -> Result<usize, String> {
        let mut chunk = vec![0u8; want.max(1)];
        let n = self.sock.read(&mut chunk).map_err(err)?;
        self.buf.extend_from_slice(&chunk[..n]);
        Ok(n)
    }

    /// Up to `max` bytes, as soon as any are available; empty at end of stream.
    pub fn read(&mut self, max: i64) -> Result<Vec<u8>, String> {
        let max = size(max)?;
        if self.buf.is_empty() {
            self.fill(max)?;
        }
        let n = max.min(self.buf.len());
        Ok(self.buf.drain(..n).collect())
    }

    /// Exactly `n` bytes; `unexpected end of stream` if the peer closes first (the bytes read so far stay buffered).
    pub fn read_exact(&mut self, n: i64) -> Result<Vec<u8>, String> {
        let n = size(n)?;
        while self.buf.len() < n {
            let need = n - self.buf.len();
            if self.fill(need.max(4096))? == 0 {
                return Err("unexpected end of stream".into());
            }
        }
        Ok(self.buf.drain(..n).collect())
    }

    /// Through the next `\n` (kept); `""` at end of stream; the last line may lack its `\n`.
    pub fn read_line(&mut self) -> Result<String, String> {
        loop {
            if let Some(i) = self.buf.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = self.buf.drain(..=i).collect();
                return String::from_utf8(line).map_err(|_| "invalid UTF-8 in line".to_string());
            }
            if self.fill(4096)? == 0 {
                let rest: Vec<u8> = std::mem::take(&mut self.buf);
                return String::from_utf8(rest).map_err(|_| "invalid UTF-8 in line".to_string());
            }
        }
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<(), String> {
        self.sock.write_all(data).map_err(err)
    }

    pub fn flush(&mut self) -> Result<(), String> {
        self.sock.flush().map_err(err)
    }

    /// Half close: the peer reads end of stream; this end can still read.
    pub fn shutdown_write(&mut self) -> Result<(), String> {
        quiet_shutdown(self.sock.shutdown(Shutdown::Write))
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        quiet_shutdown(self.sock.shutdown(Shutdown::Both))
    }

    /// Read and write timeouts in milliseconds; zero or less = none.
    pub fn set_timeouts(&mut self, read_ms: i64, write_ms: i64) -> Result<(), String> {
        self.sock.set_read_timeout(timeout_of(read_ms)).map_err(err)?;
        self.sock.set_write_timeout(timeout_of(write_ms)).map_err(err)
    }

    /// Would one read of the socket return now (data, end of stream or an error)? Never blocks. Bytes already buffered do
    /// not count: `buffered` shows them.
    pub fn ready(&self) -> bool {
        if self.sock.set_nonblocking(true).is_err() {
            return true;
        }
        let r = match self.sock.peek(&mut [0u8; 1]) {
            Ok(_) => true,
            Err(e) => e.kind() != io::ErrorKind::WouldBlock,
        };
        let _ = self.sock.set_nonblocking(false);
        r
    }

    /// One read into the buffer (call it when `ready`); 0 = end of stream.
    pub fn pump(&mut self) -> Result<usize, String> {
        self.fill(4096)
    }

    /// The bytes read but not yet taken.
    pub fn buffered(&self) -> &[u8] {
        &self.buf
    }

    pub fn read_timeout(&self) -> Option<Duration> {
        self.sock.read_timeout().ok().flatten()
    }

    pub fn peer_addr(&self) -> String {
        self.sock.peer_addr().map(|a| a.to_string()).unwrap_or_default()
    }

    pub fn local_addr(&self) -> String {
        self.sock.local_addr().map(|a| a.to_string()).unwrap_or_default()
    }
}

/// A bound TCP listener.
pub struct Listener {
    sock: std::net::TcpListener,
    /// A connection taken by `ready` (a non-blocking accept), handed out by the next `accept`.
    pending: std::sync::Mutex<Option<std::net::TcpStream>>,
}

impl Listener {
    pub fn bind(addr: &str) -> Result<Listener, String> {
        let addrs = resolve_once(addr)?;
        std::net::TcpListener::bind(&addrs[..]).map(|sock| Listener { sock, pending: Default::default() }).map_err(err)
    }

    pub fn accept(&self) -> Result<Stream, String> {
        if let Some(sock) = self.pending.lock().map(|mut p| p.take()).unwrap_or(None) {
            return Ok(Stream { sock, buf: Vec::new() });
        }
        self.sock.accept().map(|(sock, _)| Stream { sock, buf: Vec::new() }).map_err(err)
    }

    /// Is a connection waiting? Never blocks (a waiting connection is taken now and kept for `accept`).
    pub fn ready(&self) -> bool {
        let Ok(mut p) = self.pending.lock() else { return true };
        if p.is_some() || self.sock.set_nonblocking(true).is_err() {
            return true;
        }
        let r = match self.sock.accept() {
            Ok((s, _)) => {
                // an accepted socket inherits non-blocking mode on some systems
                let _ = s.set_nonblocking(false);
                *p = Some(s);
                true
            }
            Err(e) => e.kind() != io::ErrorKind::WouldBlock,
        };
        let _ = self.sock.set_nonblocking(false);
        r
    }

    pub fn local_addr(&self) -> String {
        self.sock.local_addr().map(|a| a.to_string()).unwrap_or_default()
    }
}

/// A bound UDP socket.
pub struct Datagrams {
    sock: std::net::UdpSocket,
}

impl Datagrams {
    pub fn bind(addr: &str) -> Result<Datagrams, String> {
        let addrs = resolve_once(addr)?;
        std::net::UdpSocket::bind(&addrs[..]).map(|sock| Datagrams { sock }).map_err(err)
    }

    /// Send one datagram to `addr` (resolved once, sent to the first address); the number of bytes sent.
    pub fn send_to(&self, data: &[u8], addr: &str) -> Result<i64, String> {
        let to = resolve_once(addr)?;
        self.sock.send_to(data, to[0]).map(|n| n as i64).map_err(err)
    }

    /// One datagram of at most `max` bytes (a longer one is truncated) and its sender.
    pub fn recv_from(&self, max: i64) -> Result<(Vec<u8>, String), String> {
        let mut b = vec![0u8; size(max)?];
        let (n, from) = self.sock.recv_from(&mut b).map_err(err)?;
        b.truncate(n);
        Ok((b, from.to_string()))
    }

    /// Is a datagram waiting? Never blocks.
    pub fn ready(&self) -> bool {
        if self.sock.set_nonblocking(true).is_err() {
            return true;
        }
        let r = match self.sock.peek_from(&mut [0u8; 1]) {
            Ok(_) => true,
            Err(e) => e.kind() != io::ErrorKind::WouldBlock,
        };
        let _ = self.sock.set_nonblocking(false);
        r
    }

    pub fn read_timeout(&self) -> Option<Duration> {
        self.sock.read_timeout().ok().flatten()
    }

    pub fn set_read_timeout(&self, ms: i64) -> Result<(), String> {
        self.sock.set_read_timeout(timeout_of(ms)).map_err(err)
    }

    pub fn local_addr(&self) -> String {
        self.sock.local_addr().map(|a| a.to_string()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_error_vocabulary_is_fixed() {
        let e = |k| describe(&io::Error::new(k, "os text"));
        assert_eq!(e(io::ErrorKind::WouldBlock), "timed out");
        assert_eq!(e(io::ErrorKind::TimedOut), "timed out");
        assert_eq!(e(io::ErrorKind::ConnectionRefused), "connection refused");
        assert_eq!(e(io::ErrorKind::ConnectionReset), "connection reset");
        assert_eq!(e(io::ErrorKind::UnexpectedEof), "unexpected end of stream");
    }

    #[test]
    fn a_line_split_across_reads_is_joined_and_bytes_after_it_stay_buffered() {
        let l = Listener::bind("127.0.0.1:0").unwrap();
        let mut c = connect_resolved(&resolve_once(&l.local_addr()).unwrap(), None).unwrap();
        let mut s = l.accept().unwrap();
        c.write_all(b"he").unwrap();
        c.write_all(b"llo\nrest").unwrap();
        c.shutdown_write().unwrap();
        assert_eq!(s.read_line().unwrap(), "hello\n");
        assert_eq!(s.read_exact(4).unwrap(), b"rest");
        assert_eq!(s.read(8).unwrap(), b"");
        assert_eq!(s.read_line().unwrap(), "");
    }

    #[test]
    fn readiness_never_blocks_and_a_taken_connection_is_kept_for_accept() {
        let l = Listener::bind("127.0.0.1:0").unwrap();
        assert!(!l.ready(), "nobody connected yet");
        let mut c = connect_resolved(&resolve_once(&l.local_addr()).unwrap(), None).unwrap();
        let mut waited = 0;
        while !l.ready() && waited < 200 {
            std::thread::sleep(Duration::from_millis(5));
            waited += 1;
        }
        let mut s = l.accept().unwrap();
        assert!(!s.ready(), "no data yet");
        c.write_all(b"x\n").unwrap();
        while !s.ready() {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(s.read_line().unwrap(), "x\n", "the accepted socket blocks again");
    }

    #[test]
    fn bad_sizes_and_addresses_are_errors() {
        assert!(resolve_once("no port here").unwrap_err().starts_with("invalid address"));
        let l = Listener::bind("127.0.0.1:0").unwrap();
        let mut c = connect_resolved(&resolve_once(&l.local_addr()).unwrap(), None).unwrap();
        assert!(c.read(0).unwrap_err().contains("at least 1"));
    }
}
