//! Regular expressions
//!
//! Windjammer's `std::regex` module maps to these functions.

use regex::Regex as RegexImpl;
use regex::RegexBuilder;

/// Re-export Regex type for use in Windjammer code
pub use regex::Regex;

/// Create a new regex (for reuse across multiple operations)
pub fn new(pattern: &str) -> Result<Regex, String> {
    RegexImpl::new(pattern).map_err(|e| e.to_string())
}

/// Compile a regex pattern (alias for new)
pub fn compile(pattern: &str) -> Result<Regex, String> {
    RegexImpl::new(pattern).map_err(|e| e.to_string())
}

/// Compile a regex pattern with flags
/// Flags: "i" = case insensitive, "m" = multiline, "s" = dot matches newline
pub fn compile_with_flags(pattern: &str, flags: &str) -> Result<Regex, String> {
    let mut builder = RegexBuilder::new(pattern);

    for flag in flags.chars() {
        match flag {
            'i' => {
                builder.case_insensitive(true);
            }
            'm' => {
                builder.multi_line(true);
            }
            's' => {
                builder.dot_matches_new_line(true);
            }
            _ => return Err(format!("Unknown regex flag: {}", flag)),
        }
    }

    builder.build().map_err(|e| e.to_string())
}

/// Escape special regex characters in a string
pub fn escape(text: &str) -> String {
    regex::escape(text)
}

/// Check if pattern matches string (compiles regex each time - use new() for better performance)
pub fn is_match(pattern: &str, text: &str) -> Result<bool, String> {
    let re = Regex::new(pattern).map_err(|e| e.to_string())?;
    Ok(re.is_match(text))
}

/// Check if regex matches string (with pre-compiled regex)
pub fn is_match_compiled(re: &Regex, text: &str) -> bool {
    re.is_match(text)
}

/// Find first match
pub fn find(pattern: &str, text: &str) -> Result<Option<String>, String> {
    let re = Regex::new(pattern).map_err(|e| e.to_string())?;
    Ok(re.find(text).map(|m| m.as_str().to_string()))
}

/// Find first match (with pre-compiled regex)
pub fn find_compiled(re: &Regex, text: &str) -> Option<String> {
    re.find(text).map(|m| m.as_str().to_string())
}

/// Find all matches
pub fn find_all(pattern: &str, text: &str) -> Result<Vec<String>, String> {
    let re = Regex::new(pattern).map_err(|e| e.to_string())?;
    Ok(re.find_iter(text).map(|m| m.as_str().to_string()).collect())
}

/// Find all matches (with pre-compiled regex)
pub fn find_all_compiled(re: &Regex, text: &str) -> Vec<String> {
    re.find_iter(text).map(|m| m.as_str().to_string()).collect()
}

/// Replace first match
pub fn replace(pattern: &str, text: &str, replacement: &str) -> Result<String, String> {
    let re = Regex::new(pattern).map_err(|e| e.to_string())?;
    Ok(re.replace(text, replacement).to_string())
}

/// Replace first match (with pre-compiled regex)
pub fn replace_compiled(re: &Regex, text: &str, replacement: &str) -> String {
    re.replace(text, replacement).to_string()
}

/// Replace all matches
pub fn replace_all(pattern: &str, text: &str, replacement: &str) -> Result<String, String> {
    let re = Regex::new(pattern).map_err(|e| e.to_string())?;
    Ok(re.replace_all(text, replacement).to_string())
}

/// Replace all matches (with pre-compiled regex)
pub fn replace_all_compiled(re: &Regex, text: &str, replacement: &str) -> String {
    re.replace_all(text, replacement).to_string()
}

/// Split string by regex pattern
pub fn split(pattern: &str, text: &str) -> Result<Vec<String>, String> {
    let re = Regex::new(pattern).map_err(|e| e.to_string())?;
    Ok(re.split(text).map(|s| s.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_match() {
        assert!(is_match(r"\d+", "123").unwrap());
        assert!(!is_match(r"\d+", "abc").unwrap());
    }

    #[test]
    fn test_find() {
        assert_eq!(find(r"\d+", "abc123def").unwrap(), Some("123".to_string()));
        assert_eq!(find(r"\d+", "abc").unwrap(), None);
    }

    #[test]
    fn test_find_all() {
        let matches = find_all(r"\d+", "a1b2c3").unwrap();
        assert_eq!(matches, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_replace() {
        assert_eq!(replace(r"\d+", "abc123def", "X").unwrap(), "abcXdef");
    }

    #[test]
    fn test_replace_all() {
        assert_eq!(replace_all(r"\d+", "a1b2c3", "X").unwrap(), "aXbXcX");
    }

    #[test]
    fn test_split() {
        let parts = split(r"\s+", "hello  world\t\tfoo").unwrap();
        assert_eq!(parts, vec!["hello", "world", "foo"]);
    }
}
