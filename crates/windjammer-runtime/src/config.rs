//! Config loading — Windjammer `std::config`.
//!
//! Format (`toml` / `yaml`) is an implementation detail of the config process.
//! Structured interchange remains `std::json` via [`to_json`].

use serde_json::Value;
use std::collections::HashMap;

fn reject_empty(text: &str, kind: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err(format!("empty {kind}"));
    }
    Ok(())
}

fn normalize_format(format: &str) -> Result<&'static str, String> {
    let lower = format.trim().to_ascii_lowercase();
    match lower.as_str() {
        "toml" => Ok("toml"),
        "yaml" | "yml" => Ok("yaml"),
        other => Err(format!("unknown config format: {other}")),
    }
}

fn parse_value(text: &str, format: &str) -> Result<Value, String> {
    let format = normalize_format(format)?;
    match format {
        "toml" => {
            reject_empty(text, "toml")?;
            let value: toml::Value =
                toml::from_str(text).map_err(|e| format!("toml parse: {e}"))?;
            serde_json::to_value(value).map_err(|e| format!("json encode: {e}"))
        }
        "yaml" => {
            reject_empty(text, "yaml")?;
            serde_yaml::from_str(text).map_err(|e| format!("yaml parse: {e}"))
        }
        _ => unreachable!(),
    }
}

/// Parse config text into JSON for `std::json` interop.
pub fn to_json(text: impl AsRef<str>, format: impl AsRef<str>) -> Result<String, String> {
    let value = parse_value(text.as_ref(), format.as_ref())?;
    serde_json::to_string(&value).map_err(|e| format!("json encode: {e}"))
}

/// Parse config text into a flat dotted-key → string map.
pub fn parse_flat(
    text: impl AsRef<str>,
    format: impl AsRef<str>,
) -> Result<HashMap<String, String>, String> {
    let value = parse_value(text.as_ref(), format.as_ref())?;
    let mut out = HashMap::new();
    flatten_value("", &value, &mut out)?;
    Ok(out)
}

fn flatten_value(prefix: &str, value: &Value, out: &mut HashMap<String, String>) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_value(&next, child, out)?;
            }
            Ok(())
        }
        Value::Null => {
            if !prefix.is_empty() {
                out.insert(prefix.to_string(), String::new());
            }
            Ok(())
        }
        Value::Bool(b) => {
            out.insert(prefix.to_string(), b.to_string());
            Ok(())
        }
        Value::Number(n) => {
            out.insert(prefix.to_string(), n.to_string());
            Ok(())
        }
        Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
            Ok(())
        }
        Value::Array(_) => {
            let compact =
                serde_json::to_string(value).map_err(|e| format!("json encode: {e}"))?;
            out.insert(prefix.to_string(), compact);
            Ok(())
        }
    }
}

/// Later map wins on shared keys. Keys only in `base` are kept.
pub fn merge(
    mut base: HashMap<String, String>,
    overlay: HashMap<String, String>,
) -> HashMap<String, String> {
    for (key, value) in overlay {
        base.insert(key, value);
    }
    base
}

/// Overlay `env_map` onto `map` only for keys that already exist.
pub fn overlay_matching(
    mut map: HashMap<String, String>,
    env_map: HashMap<String, String>,
) -> HashMap<String, String> {
    for (key, value) in env_map {
        if map.contains_key(&key) {
            map.insert(key, value);
        }
    }
    map
}

/// defaults < file < matching env keys.
pub fn resolve(
    defaults: HashMap<String, String>,
    file: HashMap<String, String>,
    env_map: HashMap<String, String>,
) -> HashMap<String, String> {
    overlay_matching(merge(defaults, file), env_map)
}

/// defaults < parsed config text < matching env keys.
pub fn resolve_text(
    defaults: HashMap<String, String>,
    text: impl AsRef<str>,
    format: impl AsRef<str>,
    env_map: HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    let file = parse_flat(text, format)?;
    Ok(resolve(defaults, file, env_map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_json_toml_simple() {
        let json = to_json("host = \"localhost\"\nport = 8080\n", "toml").unwrap();
        assert!(json.contains("localhost"));
        assert!(json.contains("8080"));
    }

    #[test]
    fn to_json_yaml_simple() {
        let json = to_json("host: localhost\nport: 8080\n", "yaml").unwrap();
        assert!(json.contains("localhost"));
    }

    #[test]
    fn parse_flat_toml_sections() {
        let map = parse_flat("HOST = \"localhost\"\n[server]\nPORT = 3000\n", "toml").unwrap();
        assert_eq!(map.get("HOST").map(String::as_str), Some("localhost"));
        assert_eq!(map.get("server.PORT").map(String::as_str), Some("3000"));
    }

    #[test]
    fn parse_flat_rejects_empty() {
        assert!(parse_flat("", "toml").is_err());
        assert!(parse_flat("   \n", "yaml").is_err());
    }

    #[test]
    fn resolve_file_then_env() {
        let mut defaults = HashMap::new();
        defaults.insert("HOST".into(), "localhost".into());
        defaults.insert("PORT".into(), "8080".into());
        defaults.insert("LOG".into(), "info".into());
        let mut file = HashMap::new();
        file.insert("PORT".into(), "3000".into());
        let mut env_map = HashMap::new();
        env_map.insert("LOG".into(), "debug".into());
        env_map.insert("IGNORED".into(), "x".into());
        let cfg = resolve(defaults, file, env_map);
        assert_eq!(cfg.get("HOST").map(String::as_str), Some("localhost"));
        assert_eq!(cfg.get("PORT").map(String::as_str), Some("3000"));
        assert_eq!(cfg.get("LOG").map(String::as_str), Some("debug"));
        assert!(!cfg.contains_key("IGNORED"));
    }

    #[test]
    fn unknown_format_errors() {
        assert!(to_json("a = 1", "ini").is_err());
    }
}
