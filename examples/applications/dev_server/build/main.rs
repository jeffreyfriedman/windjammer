use std::fmt::Write;

use std::fs;
use windjammer_runtime::mime;
fn main() {
    let port = 8080;
    let addr = {
        let mut __s = String::with_capacity(64);
        write!(&mut __s, "127.0.0.1:{}", port).unwrap();
        __s
    };
    println!("🚀 Windjammer Dev Server");
    println!("🌐 Listening on: http://localhost:{}", port);
    println!("Press Ctrl+C to stop");
    let result = http::serve_fn(&addr, handle_request);
    if result.is_err() {
        println!("Error starting server: {}", result.unwrap_err());
    }
}

fn handle_request(req: &http::Request) -> http::ServerResponse {
    let mut path = req.path();
    if path == "/" {
        path = String::from("/index.html");
    }
    if path.starts_with("/") {
        path = path.trim_start_matches("/");
    }
    let result = fs::read_to_string(path.clone());
    if result.is_ok() {
        let content = result.unwrap();
        let mime_type = mime::from_path(path);
        let mut response = http::ServerResponse::ok(content);
        response.headers.insert("Content-Type".to_string(), mime_type);
        response.headers.insert("Cache-Control".to_string(), "no-cache");
        response
    } else {
        http::ServerResponse::not_found()
    }
}

