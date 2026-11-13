//! `http::serve_fn` hands every GET path to its one handler, `/` included (axum's `/*path` needs a non-empty tail,
//! so `/` is routed separately): a file server answers its index. The interpreter's `serve_fn` does the same
//! (tests/interpreter_http_server_test.rs in the compiler).
#![cfg(feature = "net")]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use windjammer_runtime::http::{serve, serve_fn, Request, Router, ServerResponse};

fn get(port: u16, path: &str) -> String {
    for _ in 0..200 {
        if let Ok(mut s) = TcpStream::connect(("127.0.0.1", port)) {
            s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
            s.write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
            let mut out = String::new();
            let _ = s.read_to_string(&mut out);
            return out;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("server did not start");
}

#[test]
fn serve_fn_routes_the_root_and_every_other_path_to_the_handler() {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    std::thread::spawn(move || {
        let _ = serve_fn(&format!("127.0.0.1:{port}"), |req: &Request| ServerResponse::ok(format!("at {}", req.path())));
    });
    let root = get(port, "/");
    assert!(root.starts_with("HTTP/1.1 200"), "{root}");
    assert!(root.ends_with("at /"), "{root}");
    let deep = get(port, "/a/b.html");
    assert!(deep.starts_with("HTTP/1.1 200") && deep.ends_with("at /a/b.html"), "{deep}");
}

/// Routes written the way Windjammer programs (and the interpreter's router) spell them, `:name` and `*name`, are
/// served: axum 0.8 panics on that spelling, so the runtime translates it to `{name}` / `{*name}`.
#[test]
fn router_paths_with_colon_and_star_segments_are_served() {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    std::thread::spawn(move || {
        let app = Router::new()
            .get("/users/:id", |req: Request| ServerResponse::ok(format!("user {}", req.path())))
            .get("/files/*rest", |req: Request| ServerResponse::ok(format!("file {}", req.path())));
        let _ = serve(&format!("127.0.0.1:{port}"), app);
    });
    let user = get(port, "/users/42");
    assert!(user.starts_with("HTTP/1.1 200") && user.ends_with("user /users/42"), "{user}");
    let file = get(port, "/files/a/b.txt");
    assert!(file.starts_with("HTTP/1.1 200") && file.ends_with("file /files/a/b.txt"), "{file}");
}
