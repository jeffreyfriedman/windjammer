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

/// Get value from object by key
pub fn get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get(key)
}

/// Get string from object by key
pub fn get_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

/// Get number from object by key  
pub fn get_number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|v| v.as_f64())
}

/// Get boolean from object by key
pub fn get_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(|v| v.as_bool())
}

/// Set value in object by key
pub fn set(value: &mut Value, key: &str, new_value: Value) -> Result<(), String> {
    if let Some(obj) = value.as_object_mut() {
        obj.insert(key.to_string(), new_value);
        Ok(())
    } else {
        Err("Value is not an object".to_string())
    }
}

/// Get length of array or object
pub fn len(value: &Value) -> usize {
    match value {
        Value::Array(arr) => arr.len(),
        Value::Object(obj) => obj.len(),
        _ => 0,
    }
}

/// Check if array or object is empty
pub fn is_empty(value: &Value) -> bool {
    len(value) == 0
}

/// Get array element by index
pub fn get_index(value: &Value, index: usize) -> Option<&Value> {
    value.get(index)
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

    #[test]
    fn test_get() {
        let json = r#"{"name": "Alice", "age": 30, "active": true}"#;
        let value = parse(json).unwrap();

        assert!(get(&value, "name").is_some());
        assert_eq!(get_string(&value, "name"), Some("Alice".to_string()));
        assert_eq!(get_number(&value, "age"), Some(30.0));
        assert_eq!(get_bool(&value, "active"), Some(true));
    }

    #[test]
    fn test_set() {
        let json = r#"{"name": "Alice"}"#;
        let mut value = parse(json).unwrap();

        let new_name = string("Bob");
        let result = set(&mut value, "name", new_name);
        assert!(result.is_ok());
        assert_eq!(get_string(&value, "name"), Some("Bob".to_string()));
    }

    #[test]
    fn test_len_is_empty() {
        let json_array = r#"[1, 2, 3, 4, 5]"#;
        let value_array = parse(json_array).unwrap();
        assert_eq!(len(&value_array), 5);
        assert!(!is_empty(&value_array));

        let json_object = r#"{"a": 1, "b": 2}"#;
        let value_object = parse(json_object).unwrap();
        assert_eq!(len(&value_object), 2);
        assert!(!is_empty(&value_object));

        let empty_array = array();
        assert_eq!(len(&empty_array), 0);
        assert!(is_empty(&empty_array));
    }
}
