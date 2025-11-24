// Windjammer HTTP Server Runtime Support
// This module is automatically included when compiling programs that use std::http::Server

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

/// Start HTTP server - called by generated Windjammer code
pub fn windjammer_http_serve<F>(
    address: String,
    port: i64,
    handler: F,
) -> Result<(), String>
where
    F: Fn(HttpServerRequest) -> HttpServerResponse + Send + Sync + 'static,
{
    let addr = format!("{}:{}", address, port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;
    
    println!("🚀 Windjammer server listening on http://{}", addr);
    println!("📍 Press Ctrl+C to stop");
    
    let handler = Arc::new(handler);
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let handler = handler.clone();
                thread::spawn(move || handle_connection(&mut stream, handler));
            }
            Err(e) => {
                eprintln!("❌ Connection failed: {}", e);
            }
        }
    }
    
    Ok(())
}

fn handle_connection<F>(stream: &mut TcpStream, handler: Arc<F>)
where
    F: Fn(HttpServerRequest) -> HttpServerResponse,
{
    match parse_http_request(stream) {
        Ok(request) => {
            let method = request.method.clone();
            let path = request.path.clone();
            
            // Call user's handler
            let response = handler(request);
            
            // Send response
            if let Err(e) = send_http_response(stream, &response) {
                eprintln!("❌ Error sending response: {}", e);
            } else {
                let status_emoji = if response.status >= 200 && response.status < 300 {
                    "✅"
                } else if response.status >= 400 {
                    "❌"
                } else {
                    "📤"
                };
                println!("{} {} {} -> {}", status_emoji, method, path, response.status);
            }
        }
        Err(e) => {
            eprintln!("❌ Error parsing request: {}", e);
            let _ = send_error_response(stream, 400, &format!("Bad Request: {}", e));
        }
    }
}

/// HTTP Server Request (matches std::http::ServerRequest)
#[derive(Debug, Clone)]
pub struct HttpServerRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// HTTP Server Response (matches std::http::ServerResponse)
#[derive(Debug, Clone)]
pub struct HttpServerResponse {
    pub status: i64,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub binary_body: Option<Vec<u8>>,
}

fn parse_http_request(stream: &mut TcpStream) -> Result<HttpServerRequest, String> {
    let mut reader = BufReader::new(stream);
    
    // Read request line
    let mut request_line = String::new();
    reader.read_line(&mut request_line)
        .map_err(|e| format!("Failed to read request line: {}", e))?;
    
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid HTTP request line".to_string());
    }
    
    let method = parts[0].to_string();
    let path = parts[1].to_string();
    
    // Read headers
    let mut headers = Vec::new();
    let mut content_length = 0;
    
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)
            .map_err(|e| format!("Failed to read header: {}", e))?;
        
        if line.trim().is_empty() {
            break;
        }
        
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            
            if key.eq_ignore_ascii_case("content-length") {
                content_length = value.parse().unwrap_or(0);
            }
            
            headers.push((key, value));
        }
    }
    
    // Read body if present
    let mut body = String::new();
    if content_length > 0 {
        let mut buffer = vec![0; content_length];
        reader.read_exact(&mut buffer)
            .map_err(|e| format!("Failed to read body: {}", e))?;
        body = String::from_utf8_lossy(&buffer).to_string();
    }
    
    Ok(HttpServerRequest {
        method,
        path,
        headers,
        body,
    })
}

fn send_http_response(stream: &mut TcpStream, response: &HttpServerResponse) -> Result<(), String> {
    let reason = match response.status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    
    // Status line
    write!(stream, "HTTP/1.1 {} {}\r\n", response.status, reason)
        .map_err(|e| format!("Failed to write status: {}", e))?;
    
    // Determine body to send
    let body_bytes = if let Some(ref binary) = response.binary_body {
        binary.as_slice()
    } else {
        response.body.as_bytes()
    };
    
    // Content-Length
    write!(stream, "Content-Length: {}\r\n", body_bytes.len())
        .map_err(|e| format!("Failed to write Content-Length: {}", e))?;
    
    // Custom headers
    for (key, value) in &response.headers {
        write!(stream, "{}: {}\r\n", key, value)
            .map_err(|e| format!("Failed to write header: {}", e))?;
    }
    
    // End of headers
    stream.write_all(b"\r\n")
        .map_err(|e| format!("Failed to write header separator: {}", e))?;
    
    // Body
    stream.write_all(body_bytes)
        .map_err(|e| format!("Failed to write body: {}", e))?;
    
    stream.flush()
        .map_err(|e| format!("Failed to flush: {}", e))?;
    
    Ok(())
}

fn send_error_response(stream: &mut TcpStream, status: i64, message: &str) -> Result<(), String> {
    let response = HttpServerResponse {
        status,
        headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
        body: message.to_string(),
        binary_body: None,
    };
    send_http_response(stream, &response)
}

