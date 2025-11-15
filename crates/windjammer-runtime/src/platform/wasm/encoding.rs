//! WASM encoding implementation
//!
//! Uses browser APIs (btoa/atob) and Rust libraries.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys;

#[cfg(target_arch = "wasm32")]
use web_sys::console;

/// Encode bytes to base64 string (uses Rust base64 crate)
pub fn base64_encode(data: Vec<u8>) -> String {
    base64::encode(&data)
}

/// Encode string to base64 (uses Rust base64 crate)
pub fn base64_encode_string(data: String) -> String {
    base64::encode(data.as_bytes())
}

/// Decode base64 string to bytes (uses Rust base64 crate)
pub fn base64_decode(data: String) -> Result<Vec<u8>, String> {
    base64::decode(&data).map_err(|e| e.to_string())
}

/// Decode base64 string to string (uses Rust base64 crate)
pub fn base64_decode_string(data: String) -> Result<String, String> {
    let bytes = base64::decode(&data).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// Encode bytes to hex string (uses Rust hex crate)
pub fn hex_encode(data: Vec<u8>) -> String {
    hex::encode(&data)
}

/// Encode string to hex (uses Rust hex crate)
pub fn hex_encode_string(data: String) -> String {
    hex::encode(data.as_bytes())
}

/// Decode hex string to bytes (uses Rust hex crate)
pub fn hex_decode(data: String) -> Result<Vec<u8>, String> {
    hex::decode(&data).map_err(|e| e.to_string())
}

/// Decode hex string to string (uses Rust hex crate)
pub fn hex_decode_string(data: String) -> Result<String, String> {
    let bytes = hex::decode(&data).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// URL encode a string (uses JavaScript encodeURIComponent)
pub fn url_encode(data: String) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        match js_sys::encode_uri_component(&data).as_string() {
            Some(encoded) => encoded,
            None => {
                console::warn_1(&"Failed to URL encode string".into());
                data
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        data
    }
}

/// URL decode a string (uses JavaScript decodeURIComponent)
pub fn url_decode(data: String) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        match js_sys::decode_uri_component(&data) {
            Ok(decoded) => decoded
                .as_string()
                .ok_or_else(|| "Failed to convert decoded value to string".to_string()),
            Err(_) => Err("Failed to decode URL component".to_string()),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(data)
    }
}

/// URL encode component (same as url_encode in browser)
pub fn url_encode_component(data: String) -> String {
    url_encode(data)
}

/// URL decode component (same as url_decode in browser)
pub fn url_decode_component(data: String) -> Result<String, String> {
    url_decode(data)
}

// Note: In the browser:
// - base64/hex encoding uses Rust crates (compiled to WASM)
// - URL encoding uses native JavaScript functions for better compatibility
// - All encoding functions work the same as native platform
