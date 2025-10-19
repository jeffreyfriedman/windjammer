//! String utilities
//!
//! Windjammer's `std::strings` module maps to these functions.

/// Convert to uppercase
pub fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

/// Convert to lowercase
pub fn to_lower(s: &str) -> String {
    s.to_lowercase()
}

/// Trim whitespace
pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

/// Split string by delimiter
pub fn split(s: &str, delimiter: &str) -> Vec<String> {
    s.split(delimiter).map(|s| s.to_string()).collect()
}

/// Join strings with delimiter
pub fn join(parts: &[String], delimiter: &str) -> String {
    parts.join(delimiter)
}

/// Check if string contains substring
pub fn contains(s: &str, substring: &str) -> bool {
    s.contains(substring)
}

/// Check if string starts with prefix
pub fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// Check if string ends with suffix
pub fn ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

/// Replace all occurrences
pub fn replace(s: &str, from: &str, to: &str) -> String {
    s.replace(from, to)
}

/// Get string length
pub fn len(s: &str) -> usize {
    s.len()
}

/// Check if string is empty
pub fn is_empty(s: &str) -> bool {
    s.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_conversion() {
        assert_eq!(to_upper("hello"), "HELLO");
        assert_eq!(to_lower("WORLD"), "world");
    }

    #[test]
    fn test_trim() {
        assert_eq!(trim("  hello  "), "hello");
    }

    #[test]
    fn test_split_join() {
        let parts = split("a,b,c", ",");
        assert_eq!(parts, vec!["a", "b", "c"]);
        assert_eq!(join(&parts, "-"), "a-b-c");
    }

    #[test]
    fn test_contains() {
        assert!(contains("hello world", "world"));
        assert!(!contains("hello", "goodbye"));
    }

    #[test]
    fn test_replace() {
        assert_eq!(replace("hello world", "world", "rust"), "hello rust");
    }
}
