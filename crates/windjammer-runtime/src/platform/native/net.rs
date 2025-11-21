/// Native implementation of std::net using reqwest
use reqwest::blocking::Client;
use std::time::Duration;

pub type NetResult<T> = Result<T, String>;

/// HTTP Response
#[derive(Debug, Clone)]
pub struct Response {
    pub status: i32,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// HTTP Request builder
#[derive(Debug, Clone)]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout: Option<i32>,
}

impl Request {
    /// Create a new GET request
    pub fn get(url: String) -> Self {
        Self {
            url,
            method: "GET".to_string(),
            headers: Vec::new(),
            body: None,
            timeout: None,
        }
    }

    /// Create a new POST request
    pub fn post(url: String, body: String) -> Self {
        Self {
            url,
            method: "POST".to_string(),
            headers: Vec::new(),
            body: Some(body),
            timeout: None,
        }
    }

    /// Add a header
    pub fn header(mut self, key: String, value: String) -> Self {
        self.headers.push((key, value));
        self
    }

    /// Set timeout in seconds
    pub fn timeout(mut self, seconds: i32) -> Self {
        self.timeout = Some(seconds);
        self
    }

    /// Send the request (blocking)
    pub fn send(self) -> NetResult<Response> {
        let client = Client::builder()
            .timeout(
                self.timeout
                    .map(|s| Duration::from_secs(s as u64))
                    .unwrap_or(Duration::from_secs(30)),
            )
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let mut request = match self.method.as_str() {
            "GET" => client.get(&self.url),
            "POST" => client.post(&self.url),
            "PUT" => client.put(&self.url),
            "DELETE" => client.delete(&self.url),
            "PATCH" => client.patch(&self.url),
            _ => return Err(format!("Unsupported HTTP method: {}", self.method)),
        };

        // Add headers
        for (key, value) in self.headers {
            request = request.header(&key, &value);
        }

        // Add body if present
        if let Some(body) = self.body {
            request = request.body(body);
        }

        // Send request
        let response = request
            .send()
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        // Extract status
        let status = response.status().as_u16() as i32;

        // Extract headers
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .filter_map(
                |(k, v): (&reqwest::header::HeaderName, &reqwest::header::HeaderValue)| {
                    v.to_str()
                        .ok()
                        .map(|v_str: &str| (k.as_str().to_string(), v_str.to_string()))
                },
            )
            .collect();

        // Extract body
        let body: String = response
            .text()
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        Ok(Response {
            status,
            headers,
            body,
        })
    }

    /// Send the request asynchronously
    pub async fn send_async(self) -> NetResult<Response> {
        let client = reqwest::Client::builder()
            .timeout(
                self.timeout
                    .map(|s| Duration::from_secs(s as u64))
                    .unwrap_or(Duration::from_secs(30)),
            )
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let mut request = match self.method.as_str() {
            "GET" => client.get(&self.url),
            "POST" => client.post(&self.url),
            "PUT" => client.put(&self.url),
            "DELETE" => client.delete(&self.url),
            "PATCH" => client.patch(&self.url),
            _ => return Err(format!("Unsupported HTTP method: {}", self.method)),
        };

        // Add headers
        for (key, value) in self.headers {
            request = request.header(&key, &value);
        }

        // Add body if present
        if let Some(body) = self.body {
            request = request.body(body);
        }

        // Send request
        let response: reqwest::Response = request
            .send()
            .await
            .map_err(|e: reqwest::Error| format!("HTTP request failed: {}", e))?;

        // Extract status
        let status = response.status().as_u16() as i32;

        // Extract headers
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .filter_map(
                |(k, v): (&reqwest::header::HeaderName, &reqwest::header::HeaderValue)| {
                    v.to_str()
                        .ok()
                        .map(|v_str: &str| (k.as_str().to_string(), v_str.to_string()))
                },
            )
            .collect();

        // Extract body
        let body: String = response
            .text()
            .await
            .map_err(|e: reqwest::Error| format!("Failed to read response body: {}", e))?;

        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

/// Simple GET request (blocking)
pub fn get(url: String) -> NetResult<Response> {
    Request::get(url).send()
}

/// Simple POST request (blocking)
pub fn post(url: String, body: String) -> NetResult<Response> {
    Request::post(url, body).send()
}

/// Async GET request
pub async fn get_async(url: String) -> NetResult<Response> {
    Request::get(url).send_async().await
}

/// Async POST request
pub async fn post_async(url: String, body: String) -> NetResult<Response> {
    Request::post(url, body).send_async().await
}

/// Download a file
pub fn download(url: String, path: String) -> NetResult<()> {
    let response = get(url)?;
    std::fs::write(&path, response.body).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

/// Upload a file
pub fn upload(url: String, path: String) -> NetResult<Response> {
    let body = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    post(url, body)
}

// WebSocket support would require tokio-tungstenite
// For now, we'll provide a stub
pub struct WebSocket;

impl WebSocket {
    pub fn connect(_url: String) -> NetResult<WebSocket> {
        Err("WebSocket support requires tokio-tungstenite (not yet implemented)".to_string())
    }

    pub fn send(&self, _message: String) -> NetResult<()> {
        Err("WebSocket not connected".to_string())
    }

    pub fn receive(&self) -> NetResult<String> {
        Err("WebSocket not connected".to_string())
    }

    pub fn close(self) -> NetResult<()> {
        Ok(())
    }
}

// ============================================================================
// HTTP SERVER IMPLEMENTATION
// ============================================================================

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

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
}

impl ServerResponse {
    pub fn new(status: i32, body: String) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body,
        }
    }

    pub fn html(body: String) -> Self {
        Self {
            status: 200,
            headers: vec![("Content-Type".to_string(), "text/html".to_string())],
            body,
        }
    }

    pub fn json(body: String) -> Self {
        Self {
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body,
        }
    }

    pub fn error(status: i32, message: String) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: message,
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

    pub fn serve<F>(self, handler: F) -> NetResult<()>
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

fn handle_connection<F>(mut stream: TcpStream, handler: &F) -> NetResult<()>
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

fn parse_request(request_str: &str) -> NetResult<ServerRequest> {
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

fn send_response(stream: &mut TcpStream, response: ServerResponse) -> NetResult<()> {
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

    // Add content length
    response_str.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    response_str.push_str("Access-Control-Allow-Origin: *\r\n");
    response_str.push_str("\r\n");
    response_str.push_str(&response.body);

    stream
        .write_all(response_str.as_bytes())
        .map_err(|e| format!("Failed to write response: {}", e))?;

    stream
        .flush()
        .map_err(|e| format!("Failed to flush stream: {}", e))?;

    Ok(())
}
