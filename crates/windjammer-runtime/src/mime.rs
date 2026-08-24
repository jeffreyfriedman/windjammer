//! MIME type detection
//!
//! Windjammer's `std::mime` module maps to these functions.

use mime_guess;
use std::path::Path;

// Common text MIME types (mirror `std/mime.wj` constants)
pub const TEXT_HTML: &str = "text/html; charset=utf-8";
pub const TEXT_PLAIN: &str = "text/plain; charset=utf-8";
pub const TEXT_CSS: &str = "text/css; charset=utf-8";
pub const TEXT_XML: &str = "text/xml; charset=utf-8";
pub const TEXT_CSV: &str = "text/csv; charset=utf-8";

pub const APPLICATION_JSON: &str = "application/json; charset=utf-8";
pub const APPLICATION_JAVASCRIPT: &str = "application/javascript; charset=utf-8";
pub const APPLICATION_WASM: &str = "application/wasm";
pub const APPLICATION_PDF: &str = "application/pdf";
pub const APPLICATION_ZIP: &str = "application/zip";
pub const APPLICATION_OCTET_STREAM: &str = "application/octet-stream";
pub const APPLICATION_XML: &str = "application/xml; charset=utf-8";

pub const IMAGE_PNG: &str = "image/png";
pub const IMAGE_JPEG: &str = "image/jpeg";
pub const IMAGE_GIF: &str = "image/gif";
pub const IMAGE_SVG: &str = "image/svg+xml";
pub const IMAGE_WEBP: &str = "image/webp";
pub const IMAGE_ICO: &str = "image/x-icon";

/// Guess MIME type from file path
pub fn from_filename<P: AsRef<Path>>(path: P) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string()
}

/// Alias for from_filename (for consistency with mime_guess API)
pub fn from_path<P: AsRef<Path>>(path: P) -> String {
    from_filename(path)
}

/// Guess MIME type from file extension
pub fn from_extension(ext: &str) -> String {
    mime_guess::from_ext(ext)
        .first_or_octet_stream()
        .to_string()
}

/// Check if MIME type is text
pub fn is_text(mime_type: &str) -> bool {
    mime_type.starts_with("text/")
}

/// Check if MIME type is image
pub fn is_image(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

/// Check if MIME type is video
pub fn is_video(mime_type: &str) -> bool {
    mime_type.starts_with("video/")
}

/// Check if MIME type is audio
pub fn is_audio(mime_type: &str) -> bool {
    mime_type.starts_with("audio/")
}

/// Check if MIME type is application
pub fn is_application(mime_type: &str) -> bool {
    mime_type.starts_with("application/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_filename() {
        assert_eq!(from_filename("test.html"), "text/html");
        assert_eq!(from_filename("test.js"), "text/javascript");
        assert_eq!(from_filename("test.json"), "application/json");
        assert_eq!(from_filename("test.png"), "image/png");
        assert_eq!(from_filename("test.wasm"), "application/wasm");
    }

    #[test]
    fn test_from_extension() {
        assert_eq!(from_extension("html"), "text/html");
        assert_eq!(from_extension("css"), "text/css");
        assert_eq!(from_extension("jpg"), "image/jpeg");
    }

    #[test]
    fn test_type_checks() {
        assert!(is_text("text/html"));
        assert!(is_image("image/png"));
        assert!(is_application("application/json"));
        assert!(!is_text("image/png"));
    }
}
