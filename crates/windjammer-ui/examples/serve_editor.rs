//! Simple HTTP server to serve the WASM editor
//! Dogfooding: Eventually this will be written in Windjammer!

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

fn main() {
    println!("🚀 Windjammer HTTP Server (Dogfooding!)");
    println!("Serving editor from: /tmp/windjammer_editor_wasm");
    println!("");
    println!("Starting server on http://localhost:8080");
    println!("Open http://localhost:8080 in your browser");
    println!("Press Ctrl+C to stop");
    println!("");

    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind to port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    if let Ok(size) = stream.read(&mut buffer) {
        let request = String::from_utf8_lossy(&buffer[..size]);

        // Parse the request line
        if let Some(first_line) = request.lines().next() {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() >= 2 {
                let path = parts[1];
                handle_request(&mut stream, path);
                return;
            }
        }
    }

    send_error(&mut stream, 400, "Bad Request");
}

fn handle_request(stream: &mut TcpStream, path: &str) {
    let base_dir = "/tmp/windjammer_editor_wasm";

    let file_path = if path == "/" {
        format!("{}/index.html", base_dir)
    } else {
        format!("{}{}", base_dir, path)
    };

    println!("📄 Serving: {}", file_path);

    match fs::read(&file_path) {
        Ok(content) => {
            let content_type = get_content_type(&file_path);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                content_type,
                content.len()
            );

            if let Err(e) = stream.write_all(response.as_bytes()) {
                eprintln!("Error writing response headers: {}", e);
                return;
            }

            if let Err(e) = stream.write_all(&content) {
                eprintln!("Error writing response body: {}", e);
            }
        }
        Err(e) => {
            eprintln!("❌ Error reading file {}: {}", file_path, e);
            send_error(stream, 404, "Not Found");
        }
    }
}

fn send_error(stream: &mut TcpStream, code: u16, message: &str) {
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        code,
        message,
        message.len(),
        message
    );
    let _ = stream.write_all(response.as_bytes());
}

fn get_content_type(path: &str) -> &'static str {
    let path = Path::new(path);
    match path.extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html",
        Some("js") => "application/javascript",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
