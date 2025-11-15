/// WASM implementation of std::net using fetch API
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{Request as JsRequest, RequestInit, Response as JsResponse};

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

    /// Send the request (blocking not available in WASM, use send_async)
    pub fn send(self) -> NetResult<Response> {
        Err("Blocking requests not available in WASM. Use send_async() instead.".to_string())
    }

    /// Send the request asynchronously
    #[cfg(target_arch = "wasm32")]
    pub async fn send_async(self) -> NetResult<Response> {
        use web_sys::window;

        let window = window().ok_or_else(|| "No window object".to_string())?;

        // Create request options
        let opts = RequestInit::new();
        opts.set_method(&self.method);

        // Add body if present
        if let Some(body) = &self.body {
            opts.set_body(&JsValue::from_str(body));
        }

        // Create request
        let request = JsRequest::new_with_str_and_init(&self.url, &opts)
            .map_err(|e| format!("Failed to create request: {:?}", e))?;

        // Add headers
        let headers = request.headers();
        for (key, value) in &self.headers {
            headers
                .set(key, value)
                .map_err(|e| format!("Failed to set header: {:?}", e))?;
        }

        // Send request
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| format!("Fetch failed: {:?}", e))?;

        let resp: JsResponse = resp_value
            .dyn_into()
            .map_err(|_| "Response is not a Response object".to_string())?;

        // Extract status
        let status = resp.status() as i32;

        // Extract headers (simplified - web_sys Headers API is complex)
        let headers = Vec::new(); // TODO: Implement header extraction

        // Extract body
        let text_promise = resp
            .text()
            .map_err(|e| format!("Failed to get response text: {:?}", e))?;
        let text_value = JsFuture::from(text_promise)
            .await
            .map_err(|e| format!("Failed to read response text: {:?}", e))?;
        let body = text_value
            .as_string()
            .ok_or_else(|| "Response text is not a string".to_string())?;

        Ok(Response {
            status,
            headers,
            body,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn send_async(self) -> NetResult<Response> {
        Err("WASM-only function called in non-WASM context".to_string())
    }
}

/// Simple GET request (not available in WASM, use get_async)
pub fn get(_url: String) -> NetResult<Response> {
    Err("Blocking requests not available in WASM. Use get_async() instead.".to_string())
}

/// Simple POST request (not available in WASM, use post_async)
pub fn post(_url: String, _body: String) -> NetResult<Response> {
    Err("Blocking requests not available in WASM. Use post_async() instead.".to_string())
}

/// Async GET request
pub async fn get_async(url: String) -> NetResult<Response> {
    Request::get(url).send_async().await
}

/// Async POST request
pub async fn post_async(url: String, body: String) -> NetResult<Response> {
    Request::post(url, body).send_async().await
}

/// Download a file (not available in WASM)
pub fn download(_url: String, _path: String) -> NetResult<()> {
    Err(
        "File download not available in WASM. Use get_async() and localStorage instead."
            .to_string(),
    )
}

/// Upload a file (not available in WASM)
pub fn upload(_url: String, _path: String) -> NetResult<Response> {
    Err(
        "File upload not available in WASM. Use post_async() with file contents instead."
            .to_string(),
    )
}

// WebSocket support using web_sys
#[cfg(target_arch = "wasm32")]
pub struct WebSocket {
    ws: web_sys::WebSocket,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct WebSocket;

#[cfg(target_arch = "wasm32")]
impl WebSocket {
    pub fn connect(url: String) -> NetResult<WebSocket> {
        let ws = web_sys::WebSocket::new(&url)
            .map_err(|e| format!("Failed to create WebSocket: {:?}", e))?;
        Ok(WebSocket { ws })
    }

    pub fn send(&self, message: String) -> NetResult<()> {
        self.ws
            .send_with_str(&message)
            .map_err(|e| format!("Failed to send message: {:?}", e))?;
        Ok(())
    }

    pub fn receive(&self) -> NetResult<String> {
        Err("WebSocket receive requires event listeners (not yet implemented)".to_string())
    }

    pub fn close(self) -> NetResult<()> {
        self.ws
            .close()
            .map_err(|e| format!("Failed to close WebSocket: {:?}", e))?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl WebSocket {
    pub fn connect(_url: String) -> NetResult<WebSocket> {
        Err("WebSocket only available in WASM".to_string())
    }

    pub fn send(&self, _message: String) -> NetResult<()> {
        Err("WebSocket not available".to_string())
    }

    pub fn receive(&self) -> NetResult<String> {
        Err("WebSocket not available".to_string())
    }

    pub fn close(self) -> NetResult<()> {
        Ok(())
    }
}
