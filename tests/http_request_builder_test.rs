//! `std::http::request(method, url)`: a request with headers, a body and a timeout, sent with `.send()`.
//!
//! Independent oracle: a plain `std::net::TcpListener` in this test plays the server (it records the bytes the
//! client sent and, for the timeout cases, never answers). The interpreted run uses the interpreter's own client,
//! the compiled run uses the runtime's reqwest client: both must report the same thing.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::io::{Read, Write};
use std::process::Command;
use std::time::{Duration, Instant};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn program(port: u16) -> String {
    format!(
        "use std::http\n\nfn main() {{\n    let url = \"http://127.0.0.1:{port}/slow\"\n    match http.request(\"GET\", url).header(\"X-Probe\", \"1\").timeout_secs(1).send() {{\n        Ok(r) => println(\"status ${{r.status_code()}}\"),\n        Err(e) => println(\"error ${{e.contains(\"timed out\")}}\"),\n    }}\n}}\n"
    )
}

/// A server that accepts one connection, keeps what the client sent and then stays silent for 4 s.
fn silent_server() -> (u16, std::thread::JoinHandle<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let h = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        let mut buf = [0u8; 2048];
        let n = s.read(&mut buf).unwrap_or(0);
        std::thread::sleep(Duration::from_secs(4));
        String::from_utf8_lossy(&buf[..n]).to_string()
    });
    (port, h)
}

fn run(args: &[&str], src: &str) -> (String, String, Duration) {
    let dir = Scratch::new("http-request");
    let f = dir.join("prog.wj");
    std::fs::write(&f, src).unwrap();
    let t = Instant::now();
    let o = Command::new(WJ).arg("run").args(args).arg(&f).current_dir(dir.path()).output().unwrap();
    (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), t.elapsed())
}

#[test]
fn interpreted_request_times_out_with_an_error_and_sends_its_header() {
    let (port, server) = silent_server();
    let (out, err, took) = run(&["--interpret", "--allow-network"], &program(port));
    assert_eq!(out, "error true\n", "{err}");
    assert!(took < Duration::from_secs(4), "the request must give up after its 1 s timeout, took {took:?}");
    let seen = server.join().unwrap();
    assert!(seen.starts_with("GET /slow HTTP/1.1"), "{seen}");
    assert!(seen.contains("X-Probe: 1"), "{seen}");
}

#[test]
fn interpreted_mock_answers_and_a_slow_mock_times_out() {
    let dir = Scratch::new("http-request-mock");
    let mock = dir.join("mock.json");
    std::fs::write(
        &mock,
        r#"[{"method":"POST","url":"http://api.test/echo","status":201,"body":"made"},
            {"method":"GET","url":"http://api.test/slow","status":200,"body":"late","delay_secs":5}]"#,
    )
    .unwrap();
    let src = "use std::http\n\nfn main() {\n    match http.request(\"post\", \"http://api.test/echo\").body(\"x\").timeout_secs(2).send() {\n        Ok(r) => println(\"${r.status_code()} ${r.text().unwrap_or(\"\")}\"),\n        Err(e) => println(\"error ${e}\"),\n    }\n    match http.request(\"GET\", \"http://api.test/slow\").timeout_secs(1).send() {\n        Ok(r) => println(\"${r.status_code()}\"),\n        Err(e) => println(\"error ${e}\"),\n    }\n    match http.request(\"GET\", \"http://api.test/slow\").timeout_secs(9).send() {\n        Ok(r) => println(\"${r.status_code()}\"),\n        Err(e) => println(\"error ${e}\"),\n    }\n}\n";
    let (out, err, _) = run(&["--interpret", "--mock-http", mock.to_str().unwrap()], src);
    assert_eq!(out, "201 made\nerror timed out\n200\n", "{err}");
}

#[test]
fn interpreted_request_is_stopped_by_the_capability_gate_by_default() {
    let src = "use std::http\n\nfn main() {\n    println(\"before\")\n    let r = http.request(\"GET\", \"http://127.0.0.1:9/\").send()\n    println(\"after\")\n}\n";
    let o = Command::new(WJ)
        .args(["run", "--interpret"])
        .arg({
            let d = Scratch::new("http-request-gate");
            let f = d.join("p.wj");
            std::fs::write(&f, src).unwrap();
            std::mem::forget(d);
            f
        })
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(126), "{err}");
    assert!(err.contains("capability denied: net.connect(127.0.0.1)"), "{err}");
}

/// The compiled program (reqwest client with a timeout) reports the same thing as the interpreted one.
#[test]
fn compiled_request_times_out_with_an_error() {
    let (port, server) = silent_server();
    // without a grant the compiled program stops at the request with the same message as the interpreter
    let (denied_out, denied_err, _) = run(&[], &program(port));
    assert!(denied_out.is_empty() && denied_err.contains("capability denied: net.connect(127.0.0.1)"), "{denied_out}{denied_err}");
    let (out, err, took) = run(&["--allow", "net.connect=127.0.0.1"], &program(port));
    assert!(out.ends_with("error true\n"), "stdout: {out}\nstderr: {err}");
    // the build is part of `took`; the request itself must not wait for the server's 4 s silence
    let seen = server.join().unwrap();
    assert!(seen.starts_with("GET /slow HTTP/1.1"), "{seen}");
    assert!(seen.to_ascii_lowercase().contains("x-probe: 1"), "{seen}");
    let _ = took;
}
