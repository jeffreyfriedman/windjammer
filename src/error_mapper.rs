use std::path::PathBuf;
use serde::Deserialize;
use colored::Colorize;
use crate::source_map::SourceMap;

/// Cargo message wrapper (top-level JSON structure)
#[derive(Debug, Deserialize)]
pub struct CargoMessage {
    pub reason: String,
    #[serde(default)]
    pub message: Option<RustcDiagnostic>,
}

/// Rust compiler diagnostic (subset of rustc JSON output)
#[derive(Debug, Deserialize)]
pub struct RustcDiagnostic {
    pub message: String,
    pub level: String, // "error", "warning", "note"
    pub spans: Vec<DiagnosticSpan>,
    #[serde(default)]
    pub code: Option<DiagnosticCode>,
}

#[derive(Debug, Deserialize)]
pub struct DiagnosticSpan {
    pub file_name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub text: Vec<DiagnosticText>,
    pub is_primary: bool,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DiagnosticText {
    pub text: String,
    pub highlight_start: usize,
    pub highlight_end: usize,
}

#[derive(Debug, Deserialize)]
pub struct DiagnosticCode {
    pub code: String,
}

/// Mapped error in Windjammer context
#[derive(Debug)]
pub struct WindjammerError {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub level: ErrorLevel,
    pub message: String,
    pub code_snippet: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ErrorLevel {
    Error,
    Warning,
    Note,
}

impl WindjammerError {
    pub fn pretty_print(&self) -> String {
        let level_str = match self.level {
            ErrorLevel::Error => "error".red().bold(),
            ErrorLevel::Warning => "warning".yellow().bold(),
            ErrorLevel::Note => "note".cyan().bold(),
        };

        let location = format!(
            "{}:{}:{}",
            self.file.display(),
            self.line,
            self.column
        );

        let mut output = format!("{}: {}\n", level_str, self.message);
        output.push_str(&format!("  {} {}\n", "-->".blue().bold(), location));

        if let Some(ref snippet) = self.code_snippet {
            output.push_str("  |\n");
            output.push_str(&format!("{:>4} | {}\n", self.line, snippet));
            output.push_str("  |\n");
        }

        output
    }
}

/// Map rustc errors to Windjammer source locations
pub fn map_rustc_errors(
    diagnostics: Vec<RustcDiagnostic>,
    source_map: &SourceMap,
) -> Vec<WindjammerError> {
    diagnostics
        .into_iter()
        .filter_map(|diag| map_single_error(diag, source_map))
        .collect()
}

fn map_single_error(
    diag: RustcDiagnostic,
    source_map: &SourceMap,
) -> Option<WindjammerError> {
    // Find the primary span
    let primary_span = diag.spans.iter().find(|s| s.is_primary)?;

    // Map Rust location to Windjammer location
    let wj_location = source_map.get_location(primary_span.line_start)?;

    // Translate error message to Windjammer terminology
    let message = translate_error_message(&diag.message);

    // Extract code snippet from the primary span
    let code_snippet = primary_span
        .text
        .first()
        .map(|t| t.text.trim().to_string());

    Some(WindjammerError {
        file: wj_location.file.clone(),
        line: wj_location.line,
        column: primary_span.column_start,
        level: match diag.level.as_str() {
            "error" => ErrorLevel::Error,
            "warning" => ErrorLevel::Warning,
            _ => ErrorLevel::Note,
        },
        message,
        code_snippet,
    })
}

/// Translate Rust compiler terminology to Windjammer terms
fn translate_error_message(rust_msg: &str) -> String {
    // Common Rust error patterns → Windjammer-friendly messages
    if rust_msg.contains("mismatched types") {
        if let Some(expected) = extract_between(rust_msg, "expected `", "`") {
            if let Some(found) = extract_between(rust_msg, "found `", "`") {
                return format!(
                    "Type mismatch: expected {}, found {}",
                    rust_type_to_windjammer(expected),
                    rust_type_to_windjammer(found)
                );
            }
        }
    }

    if rust_msg.contains("cannot find type") {
        if let Some(type_name) = extract_between(rust_msg, "cannot find type `", "`") {
            return format!("Type not found: {}", type_name);
        }
    }

    if rust_msg.contains("cannot find function") {
        if let Some(func_name) = extract_between(rust_msg, "cannot find function `", "`") {
            return format!("Function not found: {}", func_name);
        }
    }

    if rust_msg.contains("cannot move out of") {
        return "Ownership error: value was moved".to_string();
    }

    if rust_msg.contains("trait bounds were not satisfied") {
        return "Missing trait implementation or type constraint".to_string();
    }

    // Fallback: return original message
    rust_msg.to_string()
}

/// Convert Rust type names to Windjammer equivalents
fn rust_type_to_windjammer(rust_type: &str) -> String {
    match rust_type {
        "i64" => "int".to_string(),
        "f64" => "float".to_string(),
        "bool" => "bool".to_string(),
        "&str" | "String" | "&String" => "string".to_string(),
        "()" => "()".to_string(),
        _ => rust_type.to_string(), // Keep unknown types as-is
    }
}

/// Extract text between two delimiters
fn extract_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_pos = text.find(start)? + start.len();
    let remaining = &text[start_pos..];
    let end_pos = remaining.find(end)?;
    Some(&remaining[..end_pos])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_type_mismatch() {
        let rust_msg = "mismatched types: expected `i64`, found `&str`";
        let translated = translate_error_message(rust_msg);
        assert_eq!(translated, "Type mismatch: expected int, found string");
    }

    #[test]
    fn test_translate_type_not_found() {
        let rust_msg = "cannot find type `Foo` in this scope";
        let translated = translate_error_message(rust_msg);
        assert_eq!(translated, "Type not found: Foo");
    }

    #[test]
    fn test_rust_type_conversion() {
        assert_eq!(rust_type_to_windjammer("i64"), "int");
        assert_eq!(rust_type_to_windjammer("f64"), "float");
        assert_eq!(rust_type_to_windjammer("&str"), "string");
        assert_eq!(rust_type_to_windjammer("String"), "string");
    }
}
