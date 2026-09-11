//! YAML parsing — Windjammer `std::yaml`.
//!
//! Parses YAML into JSON text for interop with `std::json` until native WJ value
//! bridging is complete.

use serde_json::Value;

fn reject_empty_yaml(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("empty yaml".into());
    }
    Ok(())
}

/// Parse YAML text into JSON string (structured round-trip via serde).
pub fn parse(text: impl AsRef<str>) -> Result<String, String> {
    let text = text.as_ref();
    reject_empty_yaml(text)?;
    let value: Value =
        serde_yaml::from_str(text).map_err(|e| format!("yaml parse: {e}"))?;
    serde_json::to_string(&value).map_err(|e| format!("json encode: {e}"))
}

/// Parse YAML and return equivalent JSON text.
pub fn to_json(text: impl AsRef<str>) -> Result<String, String> {
    parse(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_mapping() {
        let json = parse("name: windjammer\nversion: 1").unwrap();
        assert!(json.contains("windjammer"));
    }

    #[test]
    fn empty_and_whitespace_rejected() {
        assert!(to_json("").is_err());
        assert!(to_json("   \n").is_err());
        assert!(parse("").is_err());
    }
}
