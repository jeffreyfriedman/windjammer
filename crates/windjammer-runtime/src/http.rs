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
}

impl ServerResponse {
    /// Create a 200 OK response
    pub fn ok(body: String) -> Self {
        Self {
            status: 200,
            body,
            headers: HashMap::new(),
        }
    }

    /// Create a JSON response
    pub fn json<T: serde::Serialize>(data: &T) -> Result<Self, String> {
        let body = serde_json::to_string(data).map_err(|e| e.to_string())?;
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(Self {
            status: 200,
            body,
            headers,
        })
    }

    /// Create a 201 Created response
    pub fn created(body: String) -> Self {
        Self {
            status: 201,
            body,
            headers: HashMap::new(),
        }
    }

    /// Create a 204 No Content response
    pub fn no_content() -> Self {
        Self {
            status: 204,
            body: String::new(),
            headers: HashMap::new(),
        }
    }

    /// Create a 400 Bad Request response
    pub fn bad_request(body: String) -> Self {
        Self {
            status: 400,
            body,
            headers: HashMap::new(),
        }
    }

    /// Create a 401 Unauthorized response
    pub fn unauthorized(body: String) -> Self {
        Self {
            status: 401,
            body,
            headers: HashMap::new(),
        }
    }

    /// Create a 403 Forbidden response
    pub fn forbidden(body: String) -> Self {
        Self {
            status: 403,
            body,
            headers: HashMap::new(),
        }
    }

    /// Create a 404 Not Found response
    pub fn not_found() -> Self {
        Self {
            status: 404,
            body: "Not Found".to_string(),
            headers: HashMap::new(),
        }
    }

    /// Create a 500 Internal Server Error response
    pub fn internal_error(body: String) -> Self {
        Self {
            status: 500,
            body,
            headers: HashMap::new(),
        }
    }

    /// Create a response with custom status
    pub fn with_status(status: u16, body: String) -> Self {
        Self {
            status,
            body,
            headers: HashMap::new(),
        }
    }

    /// Add a header to the response
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}

impl IntoResponse for ServerResponse {
    fn into_response(self) -> AxumResponse {
        let mut response = (
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            self.body,
        )
            .into_response();

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
        F: Fn(Request) -> ServerResponse + Clone + Send + 'static,
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
        F: Fn(Request) -> ServerResponse + Clone + Send + 'static,
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
        F: Fn(Request) -> ServerResponse + Clone + Send + 'static,
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
        F: Fn(Request) -> ServerResponse + Clone + Send + 'static,
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
        F: Fn(Request) -> ServerResponse + Clone + Send + 'static,
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

/// Extract our Request type from Axum's request
async fn extract_request(req: AxumRequest) -> Request {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Extract query parameters
    let query: HashMap<String, String> = req
        .uri()
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
        .unwrap_or_default();

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
    F: Fn(Request) -> ServerResponse + Clone + Send + 'static,
{
    let router = Router::new().get("/*path", handler);
    serve(addr, router)
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
}
