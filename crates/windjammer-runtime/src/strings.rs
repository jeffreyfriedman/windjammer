//! String utilities
//!
//! Windjammer's `std::strings` module maps to these functions.

/// Convert to uppercase
pub fn to_upper<S: AsRef<str>>(s: S) -> String {
    s.as_ref().to_uppercase()
}

/// Convert to lowercase
pub fn to_lower<S: AsRef<str>>(s: S) -> String {
    s.as_ref().to_lowercase()
}

/// Trim whitespace
pub fn trim<S: AsRef<str>>(s: S) -> String {
    s.as_ref().trim().to_string()
}

/// Split string by delimiter
pub fn split<S: AsRef<str>>(s: S, delimiter: &str) -> Vec<String> {
    s.as_ref()
        .split(delimiter)
        .map(|s| s.to_string())
        .collect()
}

/// Join strings with delimiter
pub fn join(parts: &[String], delimiter: &str) -> String {
    parts.join(delimiter)
}

/// Check if string contains substring
pub fn contains<S: AsRef<str>>(s: S, substring: &str) -> bool {
    s.as_ref().contains(substring)
}

/// Check if string starts with prefix
pub fn starts_with<S: AsRef<str>>(s: S, prefix: &str) -> bool {
    s.as_ref().starts_with(prefix)
}

/// Check if string ends with suffix
pub fn ends_with<S: AsRef<str>>(s: S, suffix: &str) -> bool {
    s.as_ref().ends_with(suffix)
}

/// Replace all occurrences
pub fn replace<S: AsRef<str>>(s: S, from: &str, to: &str) -> String {
    s.as_ref().replace(from, to)
}

/// Get substring from start to end index (exclusive)
pub fn substring<S: AsRef<str>>(s: S, start: usize, end: usize) -> String {
    s.as_ref()
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

/// Get string length
pub fn len<S: AsRef<str>>(s: S) -> usize {
    s.as_ref().len()
}

/// Check if string is empty
pub fn is_empty<S: AsRef<str>>(s: S) -> bool {
    s.as_ref().is_empty()
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
