//! URL parse / format / join — Windjammer `std::url`.
//!
//! Boundary signatures are scanned from these `pub fn` lines (`&str` / `&Url`).
//! Implementation uses the `url` crate already in this package's Cargo.toml.

/// Parsed URL parts. Field names match `std::url.Url` / ecosystem helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

fn from_parsed(parsed: url::Url) -> Url {
    Url {
        scheme: parsed.scheme().to_string(),
        host: parsed.host_str().unwrap_or("").to_string(),
        path: parsed.path().to_string(),
        query: parsed.query().map(str::to_string),
        fragment: parsed.fragment().map(str::to_string),
    }
}

/// Parse an absolute or relative-with-base URL string.
pub fn parse(text: &str) -> Result<Url, String> {
    let parsed = url::Url::parse(text).map_err(|e| e.to_string())?;
    Ok(from_parsed(parsed))
}

/// Reconstruct a URL string from parsed parts.
pub fn format(u: &Url) -> String {
    let mut out = String::new();
    if !u.scheme.is_empty() {
        out.push_str(&u.scheme);
        out.push_str("://");
    }
    out.push_str(&u.host);
    if u.path.is_empty() && !u.host.is_empty() {
        out.push('/');
    } else {
        out.push_str(&u.path);
    }
    if let Some(q) = &u.query {
        out.push('?');
        out.push_str(q);
    }
    if let Some(f) = &u.fragment {
        out.push('#');
        out.push_str(f);
    }
    out
}

/// Resolve `rel` against `base` (RFC 3986 join).
pub fn join(base: &str, rel: &str) -> Result<String, String> {
    let parsed = url::Url::parse(base).map_err(|e| e.to_string())?;
    let joined = parsed.join(rel).map_err(|e| e.to_string())?;
    Ok(joined.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_host() {
        let u = parse("https://example.com/path?q=1#top").unwrap();
        assert_eq!(u.scheme, "https");
        assert_eq!(u.host, "example.com");
        assert_eq!(u.path, "/path");
        assert_eq!(u.query.as_deref(), Some("q=1"));
        assert_eq!(u.fragment.as_deref(), Some("top"));
    }

    #[test]
    fn format_roundtrips_host_path() {
        let u = parse("https://example.com/a").unwrap();
        assert!(format(&u).contains("example.com"));
        assert!(format(&u).contains("/a"));
    }

    #[test]
    fn join_relative_path() {
        let out = join("https://example.com/dir/", "page").unwrap();
        assert_eq!(out, "https://example.com/dir/page");
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(parse("").is_err());
    }
}
