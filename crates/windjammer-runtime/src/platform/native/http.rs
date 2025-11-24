/// Native implementation of std::http using axum
use axum::{
    body::Body,
    extract::Request as AxumRequest,
    http::{HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
    Router,
};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

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
// HTTP SERVER IMPLEMENTATION (axum-based)
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
    pub status: i64,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub binary_body: Option<Vec<u8>>,
}

impl ServerResponse {
    pub fn new(status: i64, body: String) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body,
            binary_body: None,
        }
    }

    pub fn binary(status: i64, data: Vec<u8>) -> Self {
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
            headers: vec![(
                "Content-Type".to_string(),
                "text/html; charset=utf-8".to_string(),
            )],
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

    pub fn error(status: i64, message: String) -> Self {
        Self {
            status,
            headers: vec![(
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            )],
            body: message,
            binary_body: None,
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }
}

/// HTTP Server
#[derive(Debug, Clone)]
pub struct Server {
    pub address: String,
    pub port: i64,
}

impl Server {
    pub fn new(address: String, port: i64) -> Self {
        Self { address, port }
    }

    pub fn serve<F>(self, handler: F) -> HttpResult<()>
    where
        F: Fn(&ServerRequest) -> ServerResponse + Send + Sync + 'static,
    {
        // Use tokio runtime to run the async server
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

        runtime.block_on(async { serve_with_axum(self.address, self.port, handler).await })
    }
}

async fn serve_with_axum<F>(address: String, port: i64, handler: F) -> HttpResult<()>
where
    F: Fn(&ServerRequest) -> ServerResponse + Send + Sync + 'static,
{
    let handler = Arc::new(handler);

    // Create a catch-all router that handles all methods and paths
    let app = Router::new().fallback(move |req: AxumRequest| {
        let handler = handler.clone();
        async move { handle_request(req, handler).await }
    });

    // Parse address
    let addr = format!("{}:{}", address, port);
    let socket_addr: SocketAddr = addr
        .parse()
        .map_err(|e| format!("Invalid address {}: {}", addr, e))?;

    println!("🚀 Windjammer server (axum) listening on http://{}", addr);
    println!("📍 Press Ctrl+C to stop");

    // Start axum server
    let listener = tokio::net::TcpListener::bind(socket_addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}

async fn handle_request<F>(axum_req: AxumRequest, handler: Arc<F>) -> impl IntoResponse
where
    F: Fn(&ServerRequest) -> ServerResponse,
{
    // Convert axum request to Windjammer request
    let method = axum_req.method().to_string();
    let path = axum_req.uri().path().to_string();

    // Extract headers
    let mut headers = Vec::new();
    for (key, value) in axum_req.headers() {
        if let Ok(value_str) = value.to_str() {
            headers.push((key.to_string(), value_str.to_string()));
        }
    }

    // Extract body
    let body_bytes = match axum::body::to_bytes(axum_req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("❌ Failed to read request body: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                format!("Failed to read body: {}", e),
            )
                .into_response();
        }
    };

    let body = String::from_utf8_lossy(&body_bytes).to_string();

    // Create Windjammer request
    let req = ServerRequest {
        method: method.clone(),
        path: path.clone(),
        headers,
        body,
    };

    // Call user's handler (pass by reference to match Windjammer's API)
    let response = handler(&req);

    // Log the request
    let status_emoji = if response.status >= 200 && response.status < 300 {
        "✅"
    } else if response.status >= 400 {
        "❌"
    } else {
        "📤"
    };
    println!(
        "{} {} {} -> {}",
        status_emoji, method, path, response.status
    );

    // Convert Windjammer response to axum response
    convert_to_axum_response(response)
}

fn convert_to_axum_response(response: ServerResponse) -> AxumResponse {
    // Map status code
    let status =
        StatusCode::from_u16(response.status as u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    // Build body
    let body = if let Some(binary) = response.binary_body {
        Body::from(binary)
    } else {
        Body::from(response.body)
    };

    // Build response with proper headers
    let mut builder = AxumResponse::builder().status(status);

    // Add all custom headers
    for (key, value) in response.headers {
        if let (Ok(header_name), Ok(header_value)) =
            (HeaderName::from_str(&key), HeaderValue::from_str(&value))
        {
            builder = builder.header(header_name, header_value);
        }
    }

    builder.body(body).unwrap_or_else(|e| {
        eprintln!("❌ Failed to build response: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
    })
}
