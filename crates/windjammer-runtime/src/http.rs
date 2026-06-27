//! HTTP client and server
//!
//! Windjammer's `std::http` module maps to these functions.

use axum::{
    extract::Request as AxumRequest,
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
    routing::{
        delete as axum_delete, get as axum_get, patch as axum_patch, post as axum_post,
        put as axum_put,
    },
    Router as AxumRouter,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::runtime::Runtime;

// ============================================================================
// CLIENT TYPES
// ============================================================================

/// HTTP Response from client requests
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
    pub headers: HashMap<String, String>,
}

impl Response {
    /// Get response body as string
    pub fn text(&self) -> Result<String, String> {
        String::from_utf8(self.body.clone()).map_err(|e| e.to_string())
    }

    /// Get response body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_slice(&self.body).map_err(|e| e.to_string())
    }

    /// Get response body as bytes
    pub fn bytes(&self) -> Vec<u8> {
        self.body.clone()
    }

    /// Get status code
    pub fn status_code(&self) -> u16 {
        self.status
    }

    /// Check if response is successful (2xx)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Get header value
    pub fn header(&self, key: &str) -> Option<String> {
        self.headers.get(key).cloned()
    }
}

// ============================================================================
// CLIENT FUNCTIONS
// ============================================================================

/// Perform a GET request
pub fn get(url: &str) -> Result<Response, String> {
    let rt = Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response.bytes().await.map_err(|e| e.to_string())?.to_vec();

        Ok(Response {
            status,
            body,
            headers,
        })
    })
}

/// Perform a POST request with JSON body
pub fn post_json<T: serde::Serialize>(url: &str, body: &T) -> Result<Response, String> {
    let rt = Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response.bytes().await.map_err(|e| e.to_string())?.to_vec();

        Ok(Response {
            status,
            body,
            headers,
        })
    })
}

/// Perform a POST request with string body
pub fn post(url: &str, body: &str) -> Result<Response, String> {
    let rt = Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response.bytes().await.map_err(|e| e.to_string())?.to_vec();

        Ok(Response {
            status,
            body,
            headers,
        })
    })
}

// ============================================================================
// SERVER TYPES
// ============================================================================

/// HTTP Request received by server
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    /// Get request method
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Get request path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get query parameter
    pub fn query_param(&self, key: &str) -> Option<String> {
        self.query.get(key).cloned()
    }

    /// Get header value
    pub fn header(&self, key: &str) -> Option<String> {
        self.headers.get(key).cloned()
    }

    /// Get body as string
    pub fn body_string(&self) -> Result<String, String> {
        String::from_utf8(self.body.clone()).map_err(|e| e.to_string())
    }

    /// Get body as JSON
    pub fn body_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_slice(&self.body).map_err(|e| e.to_string())
    }
}

/// HTTP Response sent by server
#[derive(Debug, Clone)]
pub struct ServerResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub binary_body: Option<Vec<u8>>,
}

impl ServerResponse {
    /// Create a response with explicit status and body.
    /// Status is Windjammer `int` (i64) — bare literals like `404` work without suffixes.
    pub fn new(status: i64, body: impl Into<String>) -> Self {
        Self {
            status: status as u16,
            body: body.into(),
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Create a 200 OK response
    pub fn ok(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            body: body.into(),
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Create a JSON response from a pre-serialized body
    pub fn json(body: impl Into<String>) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Self {
            status: 200,
            body: body.into(),
            headers,
            binary_body: None,
        }
    }

    /// Create a JSON response from a serializable value
    pub fn json_value<T: serde::Serialize>(data: &T) -> Result<Self, String> {
        let body = serde_json::to_string(data).map_err(|e| e.to_string())?;
        Ok(Self::json(body))
    }

    /// Create a 201 Created response
    pub fn created(body: String) -> Self {
        Self {
            status: 201,
            body,
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Create a 204 No Content response
    pub fn no_content() -> Self {
        Self {
            status: 204,
            body: String::new(),
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Create a 400 Bad Request response
    pub fn bad_request(body: String) -> Self {
        Self {
            status: 400,
            body,
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Create a 401 Unauthorized response
    pub fn unauthorized(body: String) -> Self {
        Self {
            status: 401,
            body,
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Create a 403 Forbidden response
    pub fn forbidden(body: String) -> Self {
        Self {
            status: 403,
            body,
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Create a 404 Not Found response
    pub fn not_found() -> Self {
        Self {
            status: 404,
            body: "Not Found".to_string(),
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Create an error response with explicit status (Windjammer `int` / i64).
    pub fn error(status: i64, message: impl Into<String>) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        Self {
            status: status as u16,
            body: message.into(),
            headers,
            binary_body: None,
        }
    }

    /// Create a binary response (WASM, images, etc.)
    pub fn binary(status: i64, data: Vec<u8>) -> Self {
        Self {
            status: status as u16,
            body: String::new(),
            headers: HashMap::new(),
            binary_body: Some(data),
        }
    }

    /// Create an HTML response
    pub fn html(body: String) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/html".to_string());
        Self {
            status: 200,
            body,
            headers,
            binary_body: None,
        }
    }

    /// Create a 500 Internal Server Error response
    pub fn internal_error(body: String) -> Self {
        Self {
            status: 500,
            body,
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Create a response with custom status
    pub fn with_status(status: u16, body: String) -> Self {
        Self {
            status,
            body,
            headers: HashMap::new(),
            binary_body: None,
        }
    }

    /// Add a header to the response (builder-style)
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add a header to the response (alias)
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}

impl IntoResponse for ServerResponse {
    fn into_response(self) -> AxumResponse {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let mut response = if let Some(binary) = self.binary_body {
            (status, binary).into_response()
        } else {
            (status, self.body).into_response()
        };

        for (key, value) in self.headers {
            if let Ok(header_name) = axum::http::HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = axum::http::HeaderValue::from_str(&value) {
                    response.headers_mut().insert(header_name, header_value);
                }
            }
        }

        response
    }
}

/// HTTP Router for building server routes
pub struct Router {
    inner: AxumRouter,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            inner: AxumRouter::new(),
        }
    }

    /// Add a GET route
    pub fn get<F>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> ServerResponse + Clone + Send + Sync + 'static,
    {
        let path = path.to_string();
        Self {
            inner: self.inner.route(
                &path,
                axum_get(move |req: AxumRequest| {
                    let handler = handler.clone();
                    async move {
                        let request = extract_request(req).await;
                        handler(request)
                    }
                }),
            ),
        }
    }

    /// Add a POST route
    pub fn post<F>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> ServerResponse + Clone + Send + Sync + 'static,
    {
        let path = path.to_string();
        Self {
            inner: self.inner.route(
                &path,
                axum_post(move |req: AxumRequest| {
                    let handler = handler.clone();
                    async move {
                        let request = extract_request(req).await;
                        handler(request)
                    }
                }),
            ),
        }
    }

    /// Add a PUT route
    pub fn put<F>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> ServerResponse + Clone + Send + Sync + 'static,
    {
        let path = path.to_string();
        Self {
            inner: self.inner.route(
                &path,
                axum_put(move |req: AxumRequest| {
                    let handler = handler.clone();
                    async move {
                        let request = extract_request(req).await;
                        handler(request)
                    }
                }),
            ),
        }
    }

    /// Add a DELETE route
    pub fn delete<F>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> ServerResponse + Clone + Send + Sync + 'static,
    {
        let path = path.to_string();
        Self {
            inner: self.inner.route(
                &path,
                axum_delete(move |req: AxumRequest| {
                    let handler = handler.clone();
                    async move {
                        let request = extract_request(req).await;
                        handler(request)
                    }
                }),
            ),
        }
    }

    /// Add a PATCH route
    pub fn patch<F>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> ServerResponse + Clone + Send + Sync + 'static,
    {
        let path = path.to_string();
        Self {
            inner: self.inner.route(
                &path,
                axum_patch(move |req: AxumRequest| {
                    let handler = handler.clone();
                    async move {
                        let request = extract_request(req).await;
                        handler(request)
                    }
                }),
            ),
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse `application/x-www-form-urlencoded` query strings (e.g. URI query component).
pub fn parse_query_string(query: Option<&str>) -> HashMap<String, String> {
    query
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Extract our Request type from Axum's request
async fn extract_request(req: AxumRequest) -> Request {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Extract query parameters
    let query = parse_query_string(req.uri().query());

    // Extract headers
    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Extract body
    let body = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .unwrap_or_default()
        .to_vec();

    Request {
        method,
        path,
        query,
        headers,
        body,
    }
}

/// Start an HTTP server
pub fn serve(addr: &str, router: Router) -> Result<(), String> {
    let rt = Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e: std::net::AddrParseError| e.to_string())?;
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| e.to_string())?;

        axum::serve(listener, router.inner)
            .await
            .map_err(|e| e.to_string())
    })
}

/// Start a simple HTTP server with a single handler function
pub fn serve_fn<F>(addr: &str, handler: F) -> Result<(), String>
where
    F: Fn(&Request) -> ServerResponse + Clone + Send + Sync + 'static,
{
    let router = Router::new().get("/*path", move |req: Request| handler(&req));
    serve(addr, router)
}

// ============================================================================
// SIMPLE SERVER API (Server::new + ServerRequest)
// ============================================================================

/// HTTP Server Request (received by simple server handlers)
#[derive(Debug, Clone)]
pub struct ServerRequest {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl ServerRequest {
    /// Lookup a single query parameter by key.
    pub fn query_param(&self, key: &str) -> Option<String> {
        self.query.get(key).cloned()
    }

    /// Windjammer std name for query parameter lookup.
    pub fn query(&self, key: &str) -> Option<String> {
        self.query_param(key)
    }

    /// Case-insensitive header lookup.
    pub fn header(&self, key: &str) -> Option<String> {
        self.headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .map(|(_, value)| value.clone())
    }
}

/// Simple HTTP server bound to address + port
#[derive(Debug, Clone)]
pub struct Server {
    pub address: String,
    pub port: i64,
}

impl Server {
    pub fn new(address: impl Into<String>, port: i64) -> Self {
        Self {
            address: address.into(),
            port,
        }
    }

    pub fn serve<F>(self, handler: F) -> Result<(), String>
    where
        F: Fn(ServerRequest) -> ServerResponse + Send + Sync + 'static,
    {
        server_serve(self.address, self.port, handler)
    }
}

/// Start a simple HTTP server with a catch-all handler
pub fn server_serve<F>(address: String, port: i64, handler: F) -> Result<(), String>
where
    F: Fn(ServerRequest) -> ServerResponse + Send + Sync + 'static,
{
    use std::sync::Arc;

    let handler = Arc::new(handler);
    let rt = Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async move {
        let handler = handler.clone();
        let app = AxumRouter::new().fallback(move |req: AxumRequest| {
            let handler = handler.clone();
            async move {
                let method = req.method().to_string();
                let path = req.uri().path().to_string();
                let query = parse_query_string(req.uri().query());
                let headers: Vec<(String, String)> = req
                    .headers()
                    .iter()
                    .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string())))
                    .collect();
                let body_bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap_or_default();
                let body = String::from_utf8_lossy(&body_bytes).to_string();
                let wj_req = ServerRequest {
                    method,
                    path,
                    query,
                    headers,
                    body,
                };
                let response = handler(wj_req);
                response.into_response()
            }
        });

        let addr = format!("{}:{}", address, port);
        let socket_addr: SocketAddr = addr
            .parse()
            .map_err(|e: std::net::AddrParseError| e.to_string())?;
        let listener = tokio::net::TcpListener::bind(socket_addr)
            .await
            .map_err(|e| e.to_string())?;
        axum::serve(listener, app).await.map_err(|e| e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_constructors() {
        let resp = ServerResponse::ok("Hello".to_string());
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "Hello");

        let resp = ServerResponse::not_found();
        assert_eq!(resp.status, 404);

        let resp = ServerResponse::with_status(418, "I'm a teapot".to_string());
        assert_eq!(resp.status, 418);
    }

    #[test]
    fn test_response_with_header() {
        let resp = ServerResponse::ok("test".to_string())
            .with_header("X-Custom".to_string(), "value".to_string());

        assert_eq!(resp.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn parse_query_string_extracts_params() {
        let q = parse_query_string(Some("as_of=2026-06-01&fmt=json"));
        assert_eq!(q.get("as_of"), Some(&"2026-06-01".to_string()));
        assert_eq!(q.get("fmt"), Some(&"json".to_string()));
    }

    #[test]
    fn server_request_query_param_lookup() {
        let mut query = HashMap::new();
        query.insert("as_of".to_string(), "2026-06-15".to_string());
        let req = ServerRequest {
            method: "GET".to_string(),
            path: "/api/v1/reports/trial-balance".to_string(),
            query,
            headers: vec![],
            body: String::new(),
        };
        assert_eq!(req.query_param("as_of"), Some("2026-06-15".to_string()));
        assert_eq!(req.query_param("missing"), None);
    }

    #[test]
    fn server_request_header_lookup_is_case_insensitive() {
        let req = ServerRequest {
            method: "GET".to_string(),
            path: "/api/v1/accounts".to_string(),
            query: HashMap::new(),
            headers: vec![("x-tenant-slug".to_string(), "demo".to_string())],
            body: String::new(),
        };
        assert_eq!(req.header("X-Tenant-Slug"), Some("demo".to_string()));
        assert_eq!(req.header("x-tenant-slug"), Some("demo".to_string()));
    }

    #[test]
    fn test_server_request_response() {
        let resp = ServerResponse::json("{\"ok\":true}".to_string());
        assert_eq!(resp.status, 200);
        assert_eq!(
            resp.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );

        let err = ServerResponse::error(404, "missing".to_string());
        assert_eq!(err.status, 404);

        let bin = ServerResponse::binary(200, vec![0x00, 0x01]);
        assert!(bin.binary_body.is_some());
    }
}
