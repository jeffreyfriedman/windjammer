//! Encoding and decoding utilities
//!
//! Windjammer's `std::encoding` module maps to these functions.

use base64::{engine::general_purpose, Engine as _};

/// Base64 encode bytes
pub fn base64_encode(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// Base64 decode string
pub fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    general_purpose::STANDARD
        .decode(s)
        .map_err(|e| e.to_string())
}

/// Hex encode bytes
pub fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Hex decode string
pub fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64() {
        let data = b"hello world";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_hex() {
        let data = b"test";
        let encoded = hex_encode(data);
        assert_eq!(encoded, "74657374");
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
