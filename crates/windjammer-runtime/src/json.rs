//! JSON serialization and deserialization
//!
//! Windjammer's `std::json` module maps to these functions.

use serde_json::Value;

/// Parse JSON string into a Value
pub fn parse(s: &str) -> Result<Value, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

/// Convert Value to JSON string
pub fn stringify(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

/// Convert Value to pretty-printed JSON string
pub fn stringify_pretty(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}

/// Create a JSON object (map)
pub fn object() -> Value {
    Value::Object(serde_json::Map::new())
}

/// Create a JSON array
pub fn array() -> Value {
    Value::Array(Vec::new())
}

/// Create a JSON null value
pub fn null() -> Value {
    Value::Null
}

/// Create a JSON boolean value
pub fn boolean(b: bool) -> Value {
    Value::Bool(b)
}

/// Create a JSON number value from i64
pub fn number_i64(n: i64) -> Value {
    Value::Number(n.into())
}

/// Create a JSON number value from f64
pub fn number_f64(n: f64) -> Result<Value, String> {
    serde_json::Number::from_f64(n)
        .map(Value::Number)
        .ok_or_else(|| "Invalid number".to_string())
}

/// Create a JSON string value
pub fn string(s: &str) -> Value {
    Value::String(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stringify() {
        let json_str = r#"{"name":"Alice","age":30}"#;
        let value = parse(json_str).unwrap();
        let result = stringify(&value).unwrap();

        // Parse both to compare (order might differ)
        let original: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_constructors() {
        assert_eq!(null(), Value::Null);
        assert_eq!(boolean(true), Value::Bool(true));
        assert_eq!(string("test"), Value::String("test".to_string()));
        assert!(matches!(number_i64(42), Value::Number(_)));
    }

    #[test]
    fn test_pretty_print() {
        let value = parse(r#"{"a":1,"b":2}"#).unwrap();
        let pretty = stringify_pretty(&value).unwrap();
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("  "));
    }
}
