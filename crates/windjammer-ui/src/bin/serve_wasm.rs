use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).expect("Failed to bind to address");

    println!("🚀 Windjammer WASM Dev Server");
    println!("📁 Root: crates/windjammer-ui");
    println!("🌐 URL: http://{}", addr);
    println!();
    println!("Available endpoints:");
    println!("  /                                  → examples/interactive_counter.html");
    println!("  /examples/interactive_counter.html → Counter demo");
    println!("  /pkg/windjammer_ui.js              → WASM JS bindings");
    println!("  /pkg/windjammer_ui_bg.wasm         → WASM binary");
    println!();
    println!("Listening... Press Ctrl+C to stop");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 8192];
    let _ = stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..]);
    let request_line = request.lines().next().unwrap_or("");

    // Parse request line: "GET /path HTTP/1.1"
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        send_response(&mut stream, 400, "text/plain", b"Bad Request");
        return;
    }

    let path = parts[1];
    println!("📥 {} {}", parts[0], path);

    // Map URL paths to file paths
    let file_path: String = match path {
        "/" => "examples/index.html".to_string(),
        "/components.css" => "examples/components.css".to_string(),
        p if p.starts_with("/examples/") => p[1..].to_string(), // Remove leading slash
        p if p.starts_with("/pkg/") => p[1..].to_string(),      // Remove leading slash
        p if p.starts_with("/pkg_counter/") => p[1..].to_string(), // Remove leading slash
        p if p.starts_with("/pkg_button_test/") => p[1..].to_string(), // Remove leading slash
        p if p.starts_with("/pkg_editor/") => p[1..].to_string(), // Remove leading slash
        p if p.starts_with("/pkg_game_editor/") => p[1..].to_string(), // Remove leading slash
        // Map root-level HTML files to examples directory
        p if p.ends_with(".html") && p.starts_with("/") && !p[1..].contains("/") => {
            format!("examples{}", p)
        }
        _ => {
            send_response(&mut stream, 404, "text/plain", b"Not Found");
            return;
        }
    };

    // Read and serve the file
    match fs::read(&file_path) {
        Ok(contents) => {
            let mime_type = get_mime_type(&file_path);
            send_response(&mut stream, 200, mime_type, &contents);
        }
        Err(e) => {
            eprintln!("❌ Error reading {}: {}", file_path, e);
            let error_msg = format!("File not found: {}", file_path);
            send_response(&mut stream, 404, "text/plain", error_msg.as_bytes());
        }
    }
}

fn send_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let status_text = match status {
        200 => "OK",
        404 => "Not Found",
        400 => "Bad Request",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        status, status_text, content_type, body.len()
    );

    stream.write_all(response.as_bytes()).unwrap();
    stream.write_all(body).unwrap();
    stream.flush().unwrap();
}

fn get_mime_type(path: &str) -> &'static str {
    let path = Path::new(path);
    match path.extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        _ => "text/plain; charset=utf-8",
    }
}
