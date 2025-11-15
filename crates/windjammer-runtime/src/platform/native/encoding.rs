//! Native encoding implementation
//!
//! Re-exports the existing windjammer-runtime encoding module.

// Re-export all functions from the parent encoding module
pub use crate::encoding::*;

// Add any additional functions needed by std::encoding API
pub fn base64_encode(data: Vec<u8>) -> String {
    base64::encode(&data)
}

pub fn base64_encode_string(data: String) -> String {
    base64::encode(data.as_bytes())
}

pub fn base64_decode(data: String) -> Result<Vec<u8>, String> {
    base64::decode(&data).map_err(|e| e.to_string())
}

pub fn base64_decode_string(data: String) -> Result<String, String> {
    let bytes = base64::decode(&data).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

pub fn hex_encode(data: Vec<u8>) -> String {
    hex::encode(&data)
}

pub fn hex_encode_string(data: String) -> String {
    hex::encode(data.as_bytes())
}

pub fn hex_decode(data: String) -> Result<Vec<u8>, String> {
    hex::decode(&data).map_err(|e| e.to_string())
}

pub fn hex_decode_string(data: String) -> Result<String, String> {
    let bytes = hex::decode(&data).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

pub fn url_encode(data: String) -> String {
    urlencoding::encode(&data).to_string()
}

pub fn url_decode(data: String) -> Result<String, String> {
    urlencoding::decode(&data)
        .map(|s| s.to_string())
        .map_err(|e| e.to_string())
}

pub fn url_encode_component(data: String) -> String {
    url_encode(data)
}

pub fn url_decode_component(data: String) -> Result<String, String> {
    url_decode(data)
}
