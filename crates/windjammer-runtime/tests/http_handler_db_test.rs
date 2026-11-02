//! A request handler that calls `std::db` must run: handlers execute on the blocking pool, not on a
//! tokio worker thread (where the database layer's `block_on` panics).
#![cfg(feature = "db")]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use windjammer_runtime::db;
use windjammer_runtime::http::{server_serve, ServerResponse};

fn get(port: u16) -> String {
    for _ in 0..100 {
        if let Ok(mut s) = TcpStream::connect(("127.0.0.1", port)) {
            s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
            s.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();
            let mut out = String::new();
            let _ = s.read_to_string(&mut out);
            return out;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("server did not start");
}

#[test]
fn handler_with_database_calls_serves_a_200() {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    std::thread::spawn(move || {
        let _ = server_serve("127.0.0.1".to_string(), port as i64, |_req| {
            let body = match db::connect(":memory:") {
                Ok(conn) => match conn.query("SELECT 1 AS one", Vec::new()) {
                    Ok(rows) => format!("rows={}", rows.len()),
                    Err(e) => format!("query error: {e}"),
                },
                Err(e) => format!("connect error: {e}"),
            };
            ServerResponse::ok(body)
        });
    });
    let reply = get(port);
    assert!(reply.starts_with("HTTP/1.1 200"), "unexpected reply: {reply}");
    assert!(reply.contains("rows=1"), "handler did not complete its query: {reply}");
}

#[test]
fn panicking_handler_answers_500_and_server_keeps_serving() {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    std::thread::spawn(move || {
        let _ = server_serve("127.0.0.1".to_string(), port as i64, |req| {
            if req.path == "/boom" {
                panic!("boom");
            }
            ServerResponse::ok("fine")
        });
    });
    let mut s = None;
    for _ in 0..100 {
        if let Ok(c) = TcpStream::connect(("127.0.0.1", port)) {
            s = Some(c);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let mut s = s.expect("server did not start");
    s.write_all(b"GET /boom HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    assert!(out.starts_with("HTTP/1.1 500"), "unexpected reply: {out}");
    assert!(get(port).contains("fine"));
}
