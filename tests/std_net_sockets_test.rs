//! Raw sockets in `std::net` (ADR-0032): one suite of programs, run interpreted, compiled, simulated and replayed.
//!
//! Independent oracles: the expected outputs are written by hand from the protocol each program speaks, and the peer
//! of the client and server programs is a Rust `std::net` socket in this test process, not Windjammer code.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const GRANT: &str = "[package]\nname = \"net-cases\"\nversion = \"0.1.0\"\n\n[capabilities]\nnet.connect = [\"127.0.0.1\"]\nnet.listen = [\"127.0.0.1\"]\n";

fn corpus(name: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/drift_corpus/net").join(name)).unwrap()
}

/// Scratch space next to the test binary (under the cargo target dir), not the shared OS temp dir.
fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let base = std::env::current_exe().unwrap().parent().unwrap().join("net-scratch");
    std::fs::create_dir_all(&base).unwrap();
    let d = tempfile::Builder::new().prefix("net-").tempdir_in(base).unwrap();
    std::fs::write(d.path().join("wj.toml"), GRANT).unwrap();
    for (n, text) in files {
        std::fs::write(d.path().join(n), text).unwrap();
    }
    d
}

fn wj(dir: &Path, flags: &[&str], file: &str, args: &[&str]) -> Output {
    Command::new(WJ).arg("run").args(flags).arg(dir.join(file)).args(args).current_dir(dir).output().unwrap()
}

fn text(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn show(o: &Output) -> String {
    format!("status {:?}\nstdout:\n{}\nstderr:\n{}", o.status.code(), text(o), String::from_utf8_lossy(&o.stderr))
}

const INTERP: &[&str] = &["--interpret", "--no-lint"];
const COMPILED: &[&str] = &["--compiled", "--no-lint"];
const SIM: &[&str] = &["--interpret", "--no-lint", "--simulate", "--seed", "7"];

/// `loopback_tcp.wj`, by hand: echo upper-cases the first line; 10 bytes split 4+6 (values 1..10); `read(2)` over 5 bytes
/// takes 3 reads; a 50 ms read timeout with no data; a closed port refuses; an address without a port is invalid.
const TCP_EXPECTED: &str = "server read hello and world
client read HELLO
peer is loopback: true
addresses agree: true
echo: ok
read_exact 4+6: first 1 last 10
read up to 64 got 2
partial reads: ok
server saw 5 bytes in 3 reads, then end of stream
client read got 5, then 0 bytes
read after shutdown: error
half close: ok
read without data: timed out
read_line without data: timed out
after the timeout the stream still works: late
timeouts: ok
connect to a closed port: connection refused
connect without a port: invalid address: 127.0.0.1
refused: ok
";

/// `loopback_udp.wj`, by hand: "ping" is 4 bytes; a 15-byte datagram read into 6 is truncated to 6; "pong" is 4.
const UDP_EXPECTED: &str = "sent 4 bytes
received 4 bytes from a: true
text ping
a 15-byte datagram read with max 6 gives 6
reply has 4 bytes
empty socket: timed out
udp: ok
";

fn loopback(file: &str, flags: &[&str], expected: &str) {
    let d = project(&[(file, &corpus(file))]);
    let o = wj(d.path(), flags, file, &[]);
    assert!(o.status.success(), "{}", show(&o));
    assert!(text(&o).ends_with(expected), "{}", show(&o));
}

#[test]
fn loopback_tcp_interpreted() {
    loopback("loopback_tcp.wj", INTERP, TCP_EXPECTED);
}

#[test]
fn loopback_tcp_simulated_on_the_virtual_network() {
    loopback("loopback_tcp.wj", SIM, TCP_EXPECTED);
}

#[test]
fn loopback_tcp_compiled() {
    loopback("loopback_tcp.wj", COMPILED, TCP_EXPECTED);
}

#[test]
fn loopback_udp_interpreted() {
    loopback("loopback_udp.wj", INTERP, UDP_EXPECTED);
}

#[test]
fn loopback_udp_simulated_on_the_virtual_network() {
    loopback("loopback_udp.wj", SIM, UDP_EXPECTED);
}

#[test]
fn loopback_udp_compiled() {
    loopback("loopback_udp.wj", COMPILED, UDP_EXPECTED);
}

/// A client of the Rust server below: greeting, a reply sent in three pieces, half close, then a silent connection.
const CLIENT: &str = r#"use std::env
use std::net::TcpStream

fn session(addr: string) -> Result<(), string> {
    let mut c = TcpStream::connect_timeout(addr, 2000)?
    c.write_str("HELLO wj\n")?
    let hi = c.read_line()?
    println("greeting ${hi.trim()}")
    let piece = c.read_exact(8)?
    match String::from_utf8(piece) {
        Ok(t) => println("assembled ${t}"),
        Err(_) => println("assembled bytes"),
    }
    c.write_str("payload")?
    c.shutdown_write()?
    let count = c.read_line()?
    println("server says ${count.trim()}")
    let end = c.read_line()?
    println("then end of stream: ${end == ""}")
    let mut quiet = TcpStream::connect(addr)?
    quiet.set_timeouts(100, 0)?
    match quiet.read(16) {
        Ok(b) => println("silent server sent ${b.len()} bytes"),
        Err(e) => println("silent server: ${e}"),
    }
    Ok(())
}

fn main() {
    let args = env::args()
    let addr = args[args.len() - 1].clone()
    match session(addr) {
        Ok(_) => println("client: ok"),
        Err(e) => println("client: FAILED ${e}"),
    }
}
"#;

const CLIENT_EXPECTED: &str = "greeting WELCOME
assembled abcdefgh
server says bytes 7
then end of stream: true
silent server: timed out
client: ok
";

/// The Rust peer of `CLIENT`: two connections, then it stops. Returns the listener's address.
fn rust_server() -> (String, std::thread::JoinHandle<()>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap().to_string();
    let h = std::thread::spawn(move || {
        let (s, _) = l.accept().unwrap();
        let mut r = BufReader::new(s.try_clone().unwrap());
        let mut w = s;
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        assert_eq!(line, "HELLO wj\n");
        w.write_all(b"WELCOME\n").unwrap();
        for part in [&b"abc"[..], b"de", b"fgh"] {
            w.write_all(part).unwrap();
            w.flush().unwrap();
            std::thread::sleep(Duration::from_millis(30));
        }
        let mut rest = Vec::new();
        r.read_to_end(&mut rest).unwrap();
        w.write_all(format!("bytes {}\n", rest.len()).as_bytes()).unwrap();
        drop(w);
        drop(r);
        let (quiet, _) = l.accept().unwrap();
        std::thread::sleep(Duration::from_millis(1500));
        drop(quiet);
    });
    (addr, h)
}

fn client_against_rust_server(flags: &[&str]) {
    let d = project(&[("client.wj", CLIENT)]);
    let (addr, h) = rust_server();
    let o = wj(d.path(), flags, "client.wj", &[&addr]);
    h.join().unwrap();
    assert!(text(&o).ends_with(CLIENT_EXPECTED), "{}", show(&o));
}

#[test]
fn client_against_a_rust_server_interpreted() {
    client_against_rust_server(INTERP);
}

#[test]
fn client_against_a_rust_server_compiled() {
    client_against_rust_server(COMPILED);
}

#[test]
fn a_recorded_session_replays_without_the_server() {
    let d = project(&[("client.wj", CLIENT)]);
    let log = d.path().join("session.log");
    let (addr, h) = rust_server();
    let rec = wj(d.path(), &["--interpret", "--no-lint", "--record", log.to_str().unwrap()], "client.wj", &[&addr]);
    h.join().unwrap();
    assert!(text(&rec).ends_with(CLIENT_EXPECTED), "{}", show(&rec));
    // the server is gone: every socket operation must come from the log
    let rep = wj(d.path(), &["--interpret", "--no-lint", "--replay", log.to_str().unwrap()], "client.wj", &[&addr]);
    assert_eq!(text(&rep), text(&rec), "{}", show(&rep));
    let logged = std::fs::read_to_string(&log).unwrap();
    assert!(logged.contains("TcpStream.read_exact") && logged.contains("TcpStream::connect_timeout"), "{logged}");
}

#[test]
fn a_simulated_run_never_reaches_the_real_network() {
    let real = TcpListener::bind("127.0.0.1:0").unwrap();
    real.set_nonblocking(true).unwrap();
    let addr = real.local_addr().unwrap().to_string();
    let d = project(&[("client.wj", CLIENT)]);
    let o = wj(d.path(), SIM, "client.wj", &[&addr]);
    assert!(text(&o).contains("client: FAILED connection refused"), "{}", show(&o));
    assert!(real.accept().is_err(), "the simulated client reached a real listener");
}

/// An upper-casing line server for the Rust client below; the port comes from the test.
const SERVER: &str = r#"use std::env
use std::net::TcpListener

fn serve(addr: string) -> Result<int, string> {
    let l = TcpListener::bind(addr)?
    let mut c = l.accept()?
    let mut n = 0
    loop {
        let line = c.read_line()?
        if line == "" {
            break
        }
        c.write_str(line.to_uppercase())?
        n = n + 1
    }
    c.write_str("bye ${n}\n")?
    Ok(n)
}

fn main() {
    let args = env::args()
    match serve(args[args.len() - 1].clone()) {
        Ok(n) => println("served ${n} lines"),
        Err(e) => println("server: FAILED ${e}"),
    }
}
"#;

fn server_against_rust_client(flags: &[&str]) {
    let d = project(&[("server.wj", SERVER)]);
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let addr = format!("127.0.0.1:{port}");
    let child = Command::new(WJ).arg("run").args(flags).arg(d.path().join("server.wj")).arg(&addr).current_dir(d.path()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
    // a compiled run builds first: keep trying for a while
    let mut s = None;
    for _ in 0..1200 {
        if let Ok(c) = TcpStream::connect(&addr) {
            s = Some(c);
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let mut s = s.expect("the Windjammer server never listened");
    s.write_all(b"one\ntwo\n").unwrap();
    s.shutdown(std::net::Shutdown::Write).unwrap();
    let mut reply = String::new();
    s.read_to_string(&mut reply).unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(reply, "ONE\nTWO\nbye 2\n", "{}", show(&o));
    assert!(text(&o).ends_with("served 2 lines\n"), "{}", show(&o));
}

#[test]
fn server_against_a_rust_client_interpreted() {
    server_against_rust_client(INTERP);
}

#[test]
fn server_against_a_rust_client_compiled() {
    server_against_rust_client(COMPILED);
}

/// Addresses the grant (127.0.0.1 only) does not cover, computed so only the run-time guard can decide.
const DENIED: &str = r#"use std::env
use std::net::TcpStream
use std::net::UdpSocket

fn main() {
    let args = env::args()
    let what = args[args.len() - 1].clone()
    let host = "10.0.0." + "1:9"
    if what == "tcp" {
        match TcpStream::connect_timeout(host, 200) {
            Ok(_) => println("connected"),
            Err(e) => println("error ${e}"),
        }
    } else {
        match UdpSocket::bind("127.0.0.1:0") {
            Ok(u) => match u.send_to("x".as_bytes().to_vec(), host) {
                Ok(_) => println("sent"),
                Err(e) => println("error ${e}"),
            },
            Err(e) => println("bind error ${e}"),
        }
    }
    println("not reached")
}
"#;

fn denied(flags: &[&str], what: &str) {
    let d = project(&[("denied.wj", DENIED)]);
    let o = wj(d.path(), flags, "denied.wj", &[what]);
    assert_eq!(o.status.code(), Some(126), "{}", show(&o));
    assert!(!text(&o).contains("not reached") && !text(&o).contains("connected") && !text(&o).contains("sent"), "{}", show(&o));
    assert!(String::from_utf8_lossy(&o.stderr).contains("capability denied"), "{}", show(&o));
}

#[test]
fn grant_denial_is_a_fault_interpreted() {
    denied(INTERP, "tcp");
    denied(INTERP, "udp");
}

#[test]
fn grant_denial_is_a_fault_compiled() {
    denied(COMPILED, "tcp");
    denied(COMPILED, "udp");
}

#[test]
fn simulated_faults_are_seeded_and_reproducible() {
    let d = project(&[("loopback_tcp.wj", &corpus("loopback_tcp.wj"))]);
    let flags = ["--interpret", "--no-lint", "--simulate", "--seed", "3", "--faults", "0.3"];
    let a = wj(d.path(), &flags, "loopback_tcp.wj", &[]);
    let b = wj(d.path(), &flags, "loopback_tcp.wj", &[]);
    assert_eq!(text(&a), text(&b), "{}", show(&a));
    assert!(text(&a).contains("FAILED connection reset") || text(&a).contains("connection refused"), "{}", show(&a));
}

/// `threaded_echo.wj`, by hand: three words upper-cased by the server thread, which then counts 3 lines.
const THREADED_EXPECTED: &str = "echo ALPHA\necho BETA\necho GAMMA\nclient: ok\nserver answered 3 lines\n";

#[test]
fn a_server_thread_runs_while_the_client_waits_interpreted() {
    loopback("threaded_echo.wj", INTERP, THREADED_EXPECTED);
}

#[test]
fn a_server_thread_runs_while_the_client_waits_simulated() {
    loopback("threaded_echo.wj", SIM, THREADED_EXPECTED);
}

#[test]
fn a_server_thread_runs_while_the_client_waits_compiled() {
    loopback("threaded_echo.wj", COMPILED, THREADED_EXPECTED);
}
