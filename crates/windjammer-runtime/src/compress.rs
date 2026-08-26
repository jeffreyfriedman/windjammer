//! Compression utilities
//!
//! Windjammer's `std::compress` module maps to these functions.
//! String APIs round-trip gzip payloads as standard Base64 so binary data
//! stays UTF-8-safe in WJ `string` values.

use base64::{engine::general_purpose, Engine as _};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};

/// Gzip-encode a UTF-8 string; returns Base64 of the compressed bytes.
pub fn gzip_encode(data: impl AsRef<str>) -> Result<String, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data.as_ref().as_bytes())
        .map_err(|e| e.to_string())?;
    let compressed = encoder.finish().map_err(|e| e.to_string())?;
    Ok(general_purpose::STANDARD.encode(compressed))
}

/// Gzip-decode a Base64 gzip payload back to a UTF-8 string.
pub fn gzip_decode(data: impl AsRef<str>) -> Result<String, String> {
    let bytes = general_purpose::STANDARD
        .decode(data.as_ref())
        .map_err(|e| e.to_string())?;
    let mut decoder = GzDecoder::new(&bytes[..]);
    let mut plain = String::new();
    decoder
        .read_to_string(&mut plain)
        .map_err(|e| e.to_string())?;
    Ok(plain)
}

/// Alias for [`gzip_decode`].
pub fn gunzip(data: impl AsRef<str>) -> Result<String, String> {
    gzip_decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_gzip() {
        let original = "hello compress world";
        let encoded = gzip_encode(original).unwrap();
        assert_ne!(encoded, original);
        let decoded = gzip_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(gunzip(&encoded).unwrap(), original);
    }
}
