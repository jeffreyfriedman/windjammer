// HTTP Server Implementation for Windjammer
// This provides a simple HTTP/1.1 server using std::net::TcpListener
// No external dependencies - pure Rust std library

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Parse an HTTP request from a TCP stream
pub fn parse_request(stream: &mut TcpStream) -> Result<(String, String, Vec<(String, String)>, String), String> {
    let mut reader = BufReader::new(stream);
    
    // Read request line (e.g., "GET /path HTTP/1.1")
    let mut request_line = String::new();
    reader.read_line(&mut request_line)
        .map_err(|e| format!("Failed to read request line: {}", e))?;
    
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid request line".to_string());
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
        
        // Empty line marks end of headers
        if line.trim().is_empty() {
            break;
        }
        
        // Parse header (e.g., "Content-Type: application/json")
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            
            // Track Content-Length for body reading
            if key.eq_ignore_ascii_case("content-length") {
                content_length = value.parse().unwrap_or(0);
            }
            
            headers.push((key, value));
        }
    }
    
    // Read body if Content-Length is specified
    let mut body = String::new();
    if content_length > 0 {
        let mut buffer = vec![0; content_length];
        reader.read_exact(&mut buffer)
            .map_err(|e| format!("Failed to read body: {}", e))?;
        body = String::from_utf8_lossy(&buffer).to_string();
    }
    
    Ok((method, path, headers, body))
}

/// Send an HTTP response to a TCP stream
pub fn send_response(
    stream: &mut TcpStream,
    status: i64,
    headers: Vec<(String, String)>,
    body: String,
    binary_body: Option<Vec<u8>>,
) -> Result<(), String> {
    // Map status code to reason phrase
    let reason = match status {
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
    
    // Write status line
    let status_line = format!("HTTP/1.1 {} {}\r\n", status, reason);
    stream.write_all(status_line.as_bytes())
        .map_err(|e| format!("Failed to write status: {}", e))?;
    
    // Determine if we're sending binary or text
    let is_binary = binary_body.is_some();
    let response_body = if is_binary {
        binary_body.as_ref().unwrap()
    } else {
        body.as_bytes()
    };
    
    // Write Content-Length header
    let content_length_header = format!("Content-Length: {}\r\n", response_body.len());
    stream.write_all(content_length_header.as_bytes())
        .map_err(|e| format!("Failed to write Content-Length: {}", e))?;
    
    // Write custom headers
    for (key, value) in headers {
        let header_line = format!("{}: {}\r\n", key, value);
        stream.write_all(header_line.as_bytes())
            .map_err(|e| format!("Failed to write header: {}", e))?;
    }
    
    // Empty line marks end of headers
    stream.write_all(b"\r\n")
        .map_err(|e| format!("Failed to write header separator: {}", e))?;
    
    // Write body
    stream.write_all(response_body)
        .map_err(|e| format!("Failed to write body: {}", e))?;
    
    stream.flush()
        .map_err(|e| format!("Failed to flush stream: {}", e))?;
    
    Ok(())
}

/// Start an HTTP server that listens on the specified address and port
/// Calls the handler function for each incoming request
pub fn start_server<F>(
    address: String,
    port: i64,
    handler: F,
) -> Result<(), String>
where
    F: Fn(String, String, Vec<(String, String)>, String) -> (i64, Vec<(String, String)>, String, Option<Vec<u8>>) + Send + Sync + 'static,
{
    let addr = format!("{}:{}", address, port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;
    
    println!("🚀 Server listening on http://{}", addr);
    
    // Wrap handler in Arc for thread sharing
    let handler = std::sync::Arc::new(handler);
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let handler = handler.clone();
                
                // Spawn a thread for each connection
                thread::spawn(move || {
                    match parse_request(&mut stream) {
                        Ok((method, path, headers, body)) => {
                            // Call the user's handler
                            let (status, resp_headers, resp_body, binary_body) = 
                                handler(method.clone(), path.clone(), headers, body);
                            
                            // Send response
                            if let Err(e) = send_response(&mut stream, status, resp_headers, resp_body, binary_body) {
                                eprintln!("❌ Error sending response: {}", e);
                            } else {
                                println!("✅ {} {} -> {}", method, path, status);
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Error parsing request: {}", e);
                            // Send 400 Bad Request
                            let _ = send_response(
                                &mut stream,
                                400,
                                vec![],
                                format!("Bad Request: {}", e),
                                None,
                            );
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("❌ Connection failed: {}", e);
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_request_line() {
        // This would require mocking TcpStream, which is complex
        // In production, test with integration tests
    }
}

