const TEXT_HTML: &'static str = "text/html; charset=utf-8";
const TEXT_PLAIN: &'static str = "text/plain; charset=utf-8";
const TEXT_CSS: &'static str = "text/css; charset=utf-8";
const TEXT_XML: &'static str = "text/xml; charset=utf-8";
const TEXT_CSV: &'static str = "text/csv; charset=utf-8";
const APPLICATION_JSON: &'static str = "application/json; charset=utf-8";
const APPLICATION_JAVASCRIPT: &'static str = "application/javascript; charset=utf-8";
const APPLICATION_WASM: &'static str = "application/wasm";
const APPLICATION_PDF: &'static str = "application/pdf";
const APPLICATION_ZIP: &'static str = "application/zip";
const APPLICATION_OCTET_STREAM: &'static str = "application/octet-stream";
const APPLICATION_XML: &'static str = "application/xml; charset=utf-8";
const IMAGE_PNG: &'static str = "image/png";
const IMAGE_JPEG: &'static str = "image/jpeg";
const IMAGE_GIF: &'static str = "image/gif";
const IMAGE_SVG: &'static str = "image/svg+xml";
const IMAGE_WEBP: &'static str = "image/webp";
const IMAGE_ICO: &'static str = "image/x-icon";
const AUDIO_MPEG: &'static str = "audio/mpeg";
const AUDIO_OGG: &'static str = "audio/ogg";
const AUDIO_WAV: &'static str = "audio/wav";
const AUDIO_WEBM: &'static str = "audio/webm";
const VIDEO_MP4: &'static str = "video/mp4";
const VIDEO_WEBM: &'static str = "video/webm";
const VIDEO_OGG: &'static str = "video/ogg";
const FONT_WOFF: &'static str = "font/woff";
const FONT_WOFF2: &'static str = "font/woff2";
const FONT_TTF: &'static str = "font/ttf";
const FONT_OTF: &'static str = "font/otf";
const APPLICATION_TYPESCRIPT: &'static str = "application/x-typescript";
const APPLICATION_SOURCEMAP: &'static str = "application/json";

#[inline]
fn from_extension(ext: &String) -> String {
    match ext.as_str() {
        "html" => TEXT_HTML,
        "htm" => TEXT_HTML,
        "txt" => TEXT_PLAIN,
        "css" => TEXT_CSS,
        "xml" => TEXT_XML,
        "csv" => TEXT_CSV,
        "js" => APPLICATION_JAVASCRIPT,
        "mjs" => APPLICATION_JAVASCRIPT,
        "json" => APPLICATION_JSON,
        "wasm" => APPLICATION_WASM,
        "pdf" => APPLICATION_PDF,
        "zip" => APPLICATION_ZIP,
        "png" => IMAGE_PNG,
        "jpg" => IMAGE_JPEG,
        "jpeg" => IMAGE_JPEG,
        "gif" => IMAGE_GIF,
        "svg" => IMAGE_SVG,
        "webp" => IMAGE_WEBP,
        "ico" => IMAGE_ICO,
        "mp3" => AUDIO_MPEG,
        "ogg" => AUDIO_OGG,
        "wav" => AUDIO_WAV,
        "mp4" => VIDEO_MP4,
        "webm" => VIDEO_WEBM,
        "ogv" => VIDEO_OGG,
        "woff" => FONT_WOFF,
        "woff2" => FONT_WOFF2,
        "ttf" => FONT_TTF,
        "otf" => FONT_OTF,
        "ts" => APPLICATION_TYPESCRIPT,
        "map" => APPLICATION_SOURCEMAP,
        _ => APPLICATION_OCTET_STREAM,
    }
}

fn from_path(path: &String) -> String {
    if path.ends_with(".html") || path.ends_with(".htm") {
        return TEXT_HTML;
    }
    if path.ends_with(".js") || path.ends_with(".mjs") {
        return APPLICATION_JAVASCRIPT;
    }
    if path.ends_with(".wasm") {
        return APPLICATION_WASM;
    }
    if path.ends_with(".css") {
        return TEXT_CSS;
    }
    if path.ends_with(".json") {
        return APPLICATION_JSON;
    }
    if path.ends_with(".png") {
        return IMAGE_PNG;
    }
    if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        return IMAGE_JPEG;
    }
    if path.ends_with(".gif") {
        return IMAGE_GIF;
    }
    if path.ends_with(".svg") {
        return IMAGE_SVG;
    }
    if path.ends_with(".webp") {
        return IMAGE_WEBP;
    }
    if path.ends_with(".ico") {
        return IMAGE_ICO;
    }
    if path.ends_with(".pdf") {
        return APPLICATION_PDF;
    }
    if path.ends_with(".zip") {
        return APPLICATION_ZIP;
    }
    if path.ends_with(".mp3") {
        return AUDIO_MPEG;
    }
    if path.ends_with(".mp4") {
        return VIDEO_MP4;
    }
    if path.ends_with(".webm") {
        return VIDEO_WEBM;
    }
    if path.ends_with(".woff") {
        return FONT_WOFF;
    }
    if path.ends_with(".woff2") {
        return FONT_WOFF2;
    }
    if path.ends_with(".ttf") {
        return FONT_TTF;
    }
    if path.ends_with(".ts") {
        return APPLICATION_TYPESCRIPT;
    }
    APPLICATION_OCTET_STREAM
}

#[inline]
fn is_text(mime_type: &String) -> bool {
    mime_type.starts_with("text/") || mime_type.starts_with("application/json") || mime_type.starts_with("application/javascript") || mime_type.starts_with("application/xml")
}

#[inline]
fn is_image(mime_type: &String) -> bool {
    mime_type.starts_with("image/")
}

#[inline]
fn is_audio(mime_type: &String) -> bool {
    mime_type.starts_with("audio/")
}

#[inline]
fn is_video(mime_type: &String) -> bool {
    mime_type.starts_with("video/")
}

