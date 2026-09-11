//! MIME type detection
//!
//! Windjammer's `std::mime` module maps to these functions.
//! Known extensions must return the same strings as `std/mime.wj` constants
//! (including `; charset=utf-8` where those constants include it).

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
pub const APPLICATION_TYPESCRIPT: &str = "application/x-typescript";
pub const APPLICATION_SOURCEMAP: &str = "application/json";

pub const IMAGE_PNG: &str = "image/png";
pub const IMAGE_JPEG: &str = "image/jpeg";
pub const IMAGE_GIF: &str = "image/gif";
pub const IMAGE_SVG: &str = "image/svg+xml";
pub const IMAGE_WEBP: &str = "image/webp";
pub const IMAGE_ICO: &str = "image/x-icon";

pub const AUDIO_MPEG: &str = "audio/mpeg";
pub const AUDIO_OGG: &str = "audio/ogg";
pub const AUDIO_WAV: &str = "audio/wav";
pub const AUDIO_WEBM: &str = "audio/webm";

pub const VIDEO_MP4: &str = "video/mp4";
pub const VIDEO_WEBM: &str = "video/webm";
pub const VIDEO_OGG: &str = "video/ogg";

pub const FONT_WOFF: &str = "font/woff";
pub const FONT_WOFF2: &str = "font/woff2";
pub const FONT_TTF: &str = "font/ttf";
pub const FONT_OTF: &str = "font/otf";

/// Canonical MIME for a known extension (no leading dot), or `None` to fall back.
fn known_extension_mime(ext: &str) -> Option<&'static str> {
    match ext {
        "html" | "htm" => Some(TEXT_HTML),
        "txt" => Some(TEXT_PLAIN),
        "css" => Some(TEXT_CSS),
        "xml" => Some(TEXT_XML),
        "csv" => Some(TEXT_CSV),
        "js" | "mjs" => Some(APPLICATION_JAVASCRIPT),
        "json" => Some(APPLICATION_JSON),
        "wasm" => Some(APPLICATION_WASM),
        "pdf" => Some(APPLICATION_PDF),
        "zip" => Some(APPLICATION_ZIP),
        "png" => Some(IMAGE_PNG),
        "jpg" | "jpeg" => Some(IMAGE_JPEG),
        "gif" => Some(IMAGE_GIF),
        "svg" => Some(IMAGE_SVG),
        "webp" => Some(IMAGE_WEBP),
        "ico" => Some(IMAGE_ICO),
        "mp3" => Some(AUDIO_MPEG),
        "ogg" => Some(AUDIO_OGG),
        "wav" => Some(AUDIO_WAV),
        "mp4" => Some(VIDEO_MP4),
        "webm" => Some(VIDEO_WEBM),
        "ogv" => Some(VIDEO_OGG),
        "woff" => Some(FONT_WOFF),
        "woff2" => Some(FONT_WOFF2),
        "ttf" => Some(FONT_TTF),
        "otf" => Some(FONT_OTF),
        "ts" => Some(APPLICATION_TYPESCRIPT),
        "map" => Some(APPLICATION_SOURCEMAP),
        _ => None,
    }
}

fn normalize_extension(ext: &str) -> String {
    ext.trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
}

/// Guess MIME type from file path
pub fn from_filename<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if let Some(mime) = known_extension_mime(&normalize_extension(ext)) {
            return mime.to_string();
        }
    }
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
    let normalized = normalize_extension(ext);
    if let Some(mime) = known_extension_mime(&normalized) {
        return mime.to_string();
    }
    mime_guess::from_ext(&normalized)
        .first_or_octet_stream()
        .to_string()
}

/// Check if MIME type is text-based (mirrors `std/mime.wj`)
pub fn is_text(mime_type: &str) -> bool {
    mime_type.starts_with("text/")
        || mime_type.starts_with("application/json")
        || mime_type.starts_with("application/javascript")
        || mime_type.starts_with("application/xml")
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
        assert_eq!(from_filename("test.html"), TEXT_HTML);
        assert_eq!(from_filename("test.js"), APPLICATION_JAVASCRIPT);
        assert_eq!(from_filename("test.json"), APPLICATION_JSON);
        assert_eq!(from_filename("test.png"), IMAGE_PNG);
        assert_eq!(from_filename("test.wasm"), APPLICATION_WASM);
    }

    #[test]
    fn test_from_extension() {
        assert_eq!(from_extension("html"), TEXT_HTML);
        assert_eq!(from_extension("css"), TEXT_CSS);
        assert_eq!(from_extension("json"), APPLICATION_JSON);
        assert_eq!(from_extension("jpg"), IMAGE_JPEG);
    }

    #[test]
    fn test_type_checks() {
        assert!(is_text(TEXT_HTML));
        assert!(is_text(APPLICATION_JSON));
        assert!(is_image(IMAGE_PNG));
        assert!(is_application(APPLICATION_JSON));
        assert!(!is_text(IMAGE_PNG));
    }
}
