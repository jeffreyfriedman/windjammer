/// Native implementation of std::http
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub type HttpResult<T> = Result<T, String>;

// ============================================================================
// HTTP CLIENT (re-export from net.rs)
// ============================================================================

pub use super::net::{get, post, Request, Response};

// Implement put and delete here
pub fn put(url: String, body: String) -> HttpResult<Response> {
    Request {
        url,
        method: "PUT".to_string(),
        headers: Vec::new(),
        body: Some(body),
        timeout: None,
    }
    .send()
}

pub fn delete(url: String) -> HttpResult<Response> {
    Request {
        url,
        method: "DELETE".to_string(),
        headers: Vec::new(),
        body: None,
        timeout: None,
    }
    .send()
}

// ============================================================================
// HTTP SERVER IMPLEMENTATION
// ============================================================================

/// HTTP Server Request (received by server)
#[derive(Debug, Clone)]
pub struct ServerRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// HTTP Server Response (sent by server)
#[derive(Debug, Clone)]
pub struct ServerResponse {
    pub status: i32,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub binary_body: Option<Vec<u8>>,
}

impl ServerResponse {
    pub fn new(status: i32, body: String) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body,
            binary_body: None,
        }
    }

    pub fn binary(status: i32, data: Vec<u8>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: String::new(),
            binary_body: Some(data),
        }
    }

    pub fn html(body: String) -> Self {
        Self {
            status: 200,
            headers: vec![("Content-Type".to_string(), "text/html".to_string())],
            body,
            binary_body: None,
        }
    }

    pub fn json(body: String) -> Self {
        Self {
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body,
            binary_body: None,
        }
    }

    pub fn error(status: i32, message: String) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: message,
            binary_body: None,
        }
    }

    pub fn header(mut self, key: String, value: String) -> Self {
        self.headers.push((key, value));
        self
    }
}

/// HTTP Server
#[derive(Debug, Clone)]
pub struct Server {
    pub address: String,
    pub port: i32,
}

impl Server {
    pub fn new(address: String, port: i32) -> Self {
        Self { address, port }
    }

    pub fn serve<F>(self, handler: F) -> HttpResult<()>
    where
        F: Fn(ServerRequest) -> ServerResponse + Send + Sync + 'static,
    {
        let addr = format!("{}:{}", self.address, self.port);
        let listener =
            TcpListener::bind(&addr).map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

        println!("🚀 Server listening on http://{}", addr);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = handle_connection(stream, &handler) {
                        eprintln!("❌ Error handling connection: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error accepting connection: {}", e);
                }
            }
        }

        Ok(())
    }
}

fn handle_connection<F>(mut stream: TcpStream, handler: &F) -> HttpResult<()>
where
    F: Fn(ServerRequest) -> ServerResponse,
{
    let mut buffer = [0; 4096];
    let size = stream
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read from stream: {}", e))?;

    let request_str = String::from_utf8_lossy(&buffer[..size]);

    // Parse request
    let request = parse_request(&request_str)?;

    println!("📄 {} {}", request.method, request.path);

    // Call handler
    let response = handler(request);

    // Send response
    send_response(&mut stream, response)?;

    Ok(())
}

fn parse_request(request_str: &str) -> HttpResult<ServerRequest> {
    let mut lines = request_str.lines();

    // Parse request line
    let first_line = lines.next().ok_or("Empty request")?;
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Err("Invalid request line".to_string());
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();

    // Parse headers
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    }

    Ok(ServerRequest {
        method,
        path,
        headers,
        body: String::new(),
    })
}

fn send_response(stream: &mut TcpStream, response: ServerResponse) -> HttpResult<()> {
    let status_text = match response.status {
        200 => "OK",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    let mut response_str = format!("HTTP/1.1 {} {}\r\n", response.status, status_text);

    // Add headers
    for (key, value) in &response.headers {
        response_str.push_str(&format!("{}: {}\r\n", key, value));
    }

    // Handle binary or text body
    if let Some(binary_data) = &response.binary_body {
        // Binary response (WASM, images, etc.)
        response_str.push_str(&format!("Content-Length: {}\r\n", binary_data.len()));
        response_str.push_str("Access-Control-Allow-Origin: *\r\n");
        response_str.push_str("\r\n");

        // Write headers
        stream
            .write_all(response_str.as_bytes())
            .map_err(|e| format!("Failed to write response headers: {}", e))?;

        // Write binary body
        stream
            .write_all(binary_data)
            .map_err(|e| format!("Failed to write binary body: {}", e))?;
    } else {
        // Text response
        response_str.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
        response_str.push_str("Access-Control-Allow-Origin: *\r\n");
        response_str.push_str("\r\n");
        response_str.push_str(&response.body);

        stream
            .write_all(response_str.as_bytes())
            .map_err(|e| format!("Failed to write response: {}", e))?;
    }

    stream
        .flush()
        .map_err(|e| format!("Failed to flush stream: {}", e))?;

    Ok(())
}
