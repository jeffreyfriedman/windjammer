#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! `std::http` servers in the interpreter, behind the client's capability port: refused by default,
//! replayed from `--mock-http`, real sockets only with `--allow-network`. Expected statuses, bodies
//! and headers are those of `windjammer_runtime::http` (axum): `ServerResponse` constructors,
//! 404 for unrouted paths, 405 for a routed path with another method. The real-socket case is
//! checked by a raw TCP client in this test (no interpreter code on the client side).

use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const SIMPLE: &str = r##"
use std::http::*

fn main() {
    let server = Server::new("127.0.0.1", PORT)
    match server.serve(handle) {
        Ok(_) => println("served"),
        Err(e) => println("ERR ${e}"),
    }
}

fn handle(request: ServerRequest) -> ServerResponse {
    let who = request.query("name").unwrap_or("anon")
    let agent = request.header("User-Agent").unwrap_or("none")
    match request.method {
        HttpMethod::POST => return ServerResponse::created("got ${request.body}"),
        _ => {}
    }
    if request.method == "GET" && request.path == "/health" {
        return ServerResponse::json("{\"status\":\"ok\"}")
    }
    if request.path == "/hi" {
        return ServerResponse::ok("hi ${who} via ${agent}").header("X-Who", who)
    }
    ServerResponse::error(404, "Not found")
}
"##;

const ROUTER: &str = r##"
use std::http::*

fn user(req: Request) -> ServerResponse {
    let v = req.query_param("v").unwrap_or("-")
    ServerResponse::ok("user ${req.path()} ${v}")
}

fn create(req: Request) -> ServerResponse {
    match req.body_string() {
        Ok(b) => ServerResponse::created(b),
        Err(e) => ServerResponse::bad_request(e),
    }
}

fn main() {
    let app = Router::new().get("/users/:id", user).post("/users", create)
    match http::serve("127.0.0.1:0", app) {
        Ok(_) => println("done"),
        Err(e) => println("ERR ${e}"),
    }
}
"##;

fn run_with(src: &str, flags: &[&str], mock: Option<&str>) -> (String, String, i32) {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let mut cmd = Command::new(WJ);
    cmd.args(["run", "--interpret"]).args(flags);
    if let Some(m) = mock {
        let p = dir.path().join("mock.json");
        fs::write(&p, m).unwrap();
        cmd.arg("--mock-http").arg(&p);
    }
    let o = cmd.arg(&f).current_dir(dir.path()).output().unwrap();
    (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code().unwrap_or(-1))
}

fn transcript(stderr: &str) -> Vec<&str> {
    stderr.lines().filter(|l| l.starts_with("[mock-http]")).collect()
}

#[test]
fn server_is_refused_without_a_capability() {
    let (out, _, code) = run_with(&SIMPLE.replace("PORT", "0"), &[], None);
    assert_eq!(code, 0);
    assert!(out.starts_with("ERR network access is disabled in the interpreter: serve 127.0.0.1:0"), "{out}");
    assert!(out.contains("--mock-http") && out.contains("--allow-network"), "{out}");
}

#[test]
fn simple_server_replays_mock_requests_in_order() {
    let mock = r#"[
 {"request": {"method": "GET", "path": "/health"}},
 {"request": {"method": "GET", "path": "/hi?name=bob", "headers": {"User-Agent": "curl"}}},
 {"request": {"method": "POST", "path": "/x", "body": "payload"}},
 {"request": {"method": "DELETE", "path": "/nope"}}
]"#;
    let (out, err, code) = run_with(&SIMPLE.replace("PORT", "0"), &[], Some(mock));
    assert_eq!((out.as_str(), code), ("served\n", 0), "{err}");
    assert_eq!(
        transcript(&err),
        [
            "[mock-http] GET /health -> 200 {\"status\":\"ok\"}",
            "[mock-http] GET /hi -> 200 hi bob via curl",
            "[mock-http] POST /x -> 201 got payload",
            "[mock-http] DELETE /nope -> 404 Not found",
        ]
    );
}

#[test]
fn router_matches_axum_routing() {
    let mock = r#"[
 {"request": {"method": "GET", "path": "/users/42?v=1"}},
 {"request": {"method": "POST", "path": "/users", "body": "{\"n\":1}"}},
 {"request": {"method": "DELETE", "path": "/users/42"}},
 {"request": {"method": "GET", "path": "/users/42/extra"}},
 {"request": {"method": "HEAD", "path": "/users/7"}}
]"#;
    let (out, err, _) = run_with(ROUTER, &[], Some(mock));
    assert_eq!(out, "done\n", "{err}");
    assert_eq!(
        transcript(&err),
        [
            "[mock-http] GET /users/42 -> 200 user /users/42 1",
            "[mock-http] POST /users -> 201 {\"n\":1}",
            "[mock-http] DELETE /users/42 -> 405 ",
            "[mock-http] GET /users/42/extra -> 404 ",
            "[mock-http] HEAD /users/7 -> 200 user /users/7 -",
        ]
    );
}

#[test]
fn methods_the_runtime_lacks_are_loud_errors() {
    let src = "use std::http::*\nfn h(r: Request) -> ServerResponse { ServerResponse::ok(\"x\") }\nfn main() {\n    let app = Router::new().any(\"/\", h)\n}\n";
    let (_, err, code) = run_with(src, &[], None);
    assert_ne!(code, 0);
    assert!(err.contains("any"), "{err}");
}

fn exchange(port: u16, raw: &str) -> String {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.write_all(raw.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn real_socket_only_with_allow_network() {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, SIMPLE.replace("PORT", &port.to_string())).unwrap();
    let mut child = Command::new(WJ)
        .args(["run", "--interpret", "--allow", "net.listen=127.0.0.1"])
        .arg(&f)
        .current_dir(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(start.elapsed() < Duration::from_secs(120), "server did not start");
        std::thread::sleep(Duration::from_millis(100));
    }
    let hi = exchange(port, "GET /hi?name=ann HTTP/1.1\r\nHost: x\r\nUser-Agent: raw\r\n\r\n");
    let post = exchange(port, "POST /p HTTP/1.1\r\nHost: x\r\nContent-Length: 3\r\n\r\nabc");
    let _ = child.kill();
    let _ = child.wait();
    assert!(hi.starts_with("HTTP/1.1 200 OK\r\n"), "{hi}");
    assert!(hi.to_ascii_lowercase().contains("x-who: ann\r\n"), "{hi}");
    assert!(hi.ends_with("\r\n\r\nhi ann via raw"), "{hi}");
    assert!(post.starts_with("HTTP/1.1 201 Created\r\n") && post.ends_with("got abc"), "{post}");
}
