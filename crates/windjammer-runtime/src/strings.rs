//! String utilities — Windjammer `std::strings` maps here.

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

/// Trim whitespace from start
pub fn trim_start<S: AsRef<str>>(s: S) -> String {
    s.as_ref().trim_start().to_string()
}

/// Trim whitespace from end
pub fn trim_end<S: AsRef<str>>(s: S) -> String {
    s.as_ref().trim_end().to_string()
}

/// Split string by delimiter
pub fn split<S: AsRef<str>>(s: S, delimiter: &str) -> Vec<String> {
    s.as_ref().split(delimiter).map(|s| s.to_string()).collect()
}

/// Split text into lines (`\n` / `\r\n` — no `.as_bytes()` needed in Windjammer).
pub fn split_lines<S: AsRef<str>>(text: S) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in text.as_ref().chars() {
        if ch == '\n' {
            lines.push(current);
            current = String::new();
        } else if ch != '\r' {
            current.push(ch);
        }
    }
    if !current.is_empty() || text.as_ref().ends_with('\n') {
        lines.push(current);
    }
    lines
}

/// Split on ASCII whitespace (space / tab).
pub fn split_whitespace<S: AsRef<str>>(line: S) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    for ch in line.as_ref().chars() {
        if ch == ' ' || ch == '\t' {
            if in_word {
                parts.push(current);
                current = String::new();
                in_word = false;
            }
        } else {
            current.push(ch);
            in_word = true;
        }
    }
    if in_word {
        parts.push(current);
    }
    parts
}

/// Parse signed integer (returns 0 on failure — matches legacy scene parser).
pub fn parse_i32<S: AsRef<str>>(s: S) -> i32 {
    s.as_ref().trim().parse().unwrap_or(0)
}

/// Parse float (returns 0.0 on failure).
pub fn parse_f32<S: AsRef<str>>(s: S) -> f32 {
    s.as_ref().trim().parse().unwrap_or(0.0)
}

/// Parse bool from `"true"` / `"false"`.
pub fn parse_bool<S: AsRef<str>>(s: S) -> bool {
    s.as_ref().trim() == "true"
}

/// ASCII byte at index (for line-oriented parsers on `.wjscene` files).
pub fn byte_at<S: AsRef<str>>(s: S, index: usize) -> u8 {
    s.as_ref().as_bytes().get(index).copied().unwrap_or(0)
}

/// Join strings with delimiter
pub fn join(parts: &[String], delimiter: &str) -> String {
    parts.join(delimiter)
}

/// Unicode characters in a string (for editor widgets, parsers, etc.).
pub fn chars<S: AsRef<str>>(s: S) -> Vec<char> {
    s.as_ref().chars().collect()
}

/// Build a string from character codepoints.
pub fn from_chars(parts: &[char]) -> String {
    parts.iter().collect()
}

/// Substring by character indices `[start, end)`.
pub fn substring_chars<S: AsRef<str>>(s: S, start: usize, end: usize) -> String {
    s.as_ref()
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
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

/// Get string length in bytes
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
    fn test_split_lines() {
        let lines = split_lines("a\nb\r\nc\n");
        assert_eq!(lines, vec!["a", "b", "c", ""]);
    }

    #[test]
    fn test_split_whitespace() {
        let parts = split_whitespace("  hello\tworld  ");
        assert_eq!(parts, vec!["hello", "world"]);
    }

    #[test]
    fn test_parse_i32() {
        assert_eq!(parse_i32("-42"), -42);
        assert_eq!(parse_i32("  7 "), 7);
        assert_eq!(parse_i32("bad"), 0);
    }

    #[test]
    fn test_parse_f32() {
        assert!((parse_f32("3.14") - 3.14).abs() < 0.001);
        assert!((parse_f32("-2.5") - (-2.5)).abs() < 0.001);
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true"));
        assert!(!parse_bool("false"));
    }

    #[test]
    fn test_byte_at() {
        assert_eq!(byte_at("ABC", 0), b'A');
        assert_eq!(byte_at("ABC", 2), b'C');
        assert_eq!(byte_at("ABC", 99), 0);
    }

    #[test]
    fn test_chars_and_from_chars() {
        let c = chars("hello");
        assert_eq!(c, vec!['h', 'e', 'l', 'l', 'o']);
        assert_eq!(from_chars(&c), "hello");
        assert_eq!(substring_chars("hello", 1, 4), "ell");
    }
}
