/// Error Mapper: Translates Rust compiler errors back to Windjammer source locations
///
/// This module intercepts rustc JSON output, maps errors using source maps,
/// and provides a world-class error experience for Windjammer developers.
use crate::error_codes;
use crate::source_map::{Location, SourceMap};
use crate::syntax_highlighter::SyntaxHighlighter;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// CARGO JSON OUTPUT STRUCTURES
// ============================================================================

/// Cargo message wrapper (from --message-format=json)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CargoMessage {
    /// Message reason (e.g., "compiler-message", "compiler-artifact")
    pub reason: String,
    /// Compiler diagnostic (if reason is "compiler-message")
    pub message: Option<RustcDiagnostic>,
}

// ============================================================================
// RUSTC JSON OUTPUT STRUCTURES
// ============================================================================

/// Rustc diagnostic message (from --error-format=json)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustcDiagnostic {
    /// Error message text
    pub message: String,
    /// Severity level (error, warning, note, help)
    pub level: String,
    /// Primary code span (file, line, column)
    pub spans: Vec<RustcSpan>,
    /// Error code (e.g., "E0308")
    pub code: Option<RustcCode>,
    /// Child diagnostics (notes, help messages)
    pub children: Vec<RustcDiagnostic>,
    /// Rendered text (for fallback display)
    pub rendered: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustcSpan {
    /// File path
    pub file_name: String,
    /// Line number (1-indexed)
    pub line_start: usize,
    pub line_end: usize,
    /// Column number (1-indexed)
    pub column_start: usize,
    pub column_end: usize,
    /// Whether this is the primary span
    pub is_primary: bool,
    /// Label text for this span
    pub label: Option<String>,
    /// Suggested replacement (for fix suggestions)
    pub suggested_replacement: Option<String>,
    /// Text content of the span (lines of source code)
    pub text: Option<Vec<RustcSpanText>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustcSpanText {
    /// Text content
    pub text: String,
    /// Whether this line is highlighted
    pub highlight_start: usize,
    pub highlight_end: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustcCode {
    /// Error code (e.g., "E0308")
    pub code: String,
    /// Explanation text
    pub explanation: Option<String>,
}

// ============================================================================
// WINDJAMMER DIAGNOSTIC STRUCTURES
// ============================================================================

/// Windjammer diagnostic message (mapped from Rust)
#[derive(Debug, Clone)]
pub struct WindjammerDiagnostic {
    /// Error message (translated to Windjammer terminology)
    pub message: String,
    /// Severity level
    pub level: DiagnosticLevel,
    /// Primary location in Windjammer source
    pub location: Location,
    /// Additional spans (for multi-location errors)
    pub spans: Vec<DiagnosticSpan>,
    /// Error code (if applicable)
    pub code: Option<String>,
    /// Help messages
    pub help: Vec<String>,
    /// Notes
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
    Help,
}

#[derive(Debug, Clone)]
pub struct DiagnosticSpan {
    /// Location in Windjammer source
    pub location: Location,
    /// Label for this span
    pub label: Option<String>,
    /// Whether this is the primary span
    pub is_primary: bool,
}

// ============================================================================
// ERROR MAPPER
// ============================================================================

pub struct ErrorMapper {
    source_map: SourceMap,
}

impl ErrorMapper {
    /// Create a new error mapper with the given source map
    pub fn new(source_map: SourceMap) -> Self {
        Self { source_map }
    }

    /// Parse rustc JSON output and map errors to Windjammer source
    pub fn map_rustc_output(&self, json_output: &str) -> Vec<WindjammerDiagnostic> {
        let mut diagnostics = Vec::new();

        for line in json_output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Parse CargoMessage first
            if let Ok(cargo_msg) = serde_json::from_str::<CargoMessage>(line) {
                // Only process compiler messages
                if cargo_msg.reason == "compiler-message" {
                    if let Some(rustc_diag) = cargo_msg.message {
                        // Only process errors and warnings
                        if rustc_diag.level == "error" || rustc_diag.level == "warning" {
                            if let Some(wj_diag) = self.map_diagnostic(&rustc_diag) {
                                diagnostics.push(wj_diag);
                            }
                        }
                    }
                }
            }
        }

        diagnostics
    }

    /// Map a single rustc diagnostic to Windjammer
    fn map_diagnostic(&self, rustc_diag: &RustcDiagnostic) -> Option<WindjammerDiagnostic> {
        // Find the primary span
        let primary_span = rustc_diag.spans.iter().find(|s| s.is_primary)?;

        // Map Rust location to Windjammer location
        let rust_location = Location {
            file: PathBuf::from(&primary_span.file_name),
            line: primary_span.line_start,
            column: primary_span.column_start,
        };

        // Try to map to Windjammer location, fallback to Rust location if no mapping exists
        let wj_location = self
            .source_map
            .map_rust_to_windjammer(&rust_location)
            .unwrap_or_else(|| {
                // No mapping found - use Rust location but try to infer Windjammer file
                // by looking for any mapping from this Rust file
                let wj_file = self
                    .source_map
                    .mappings_for_rust_file(&rust_location.file)
                    .first()
                    .map(|m| m.wj_file.clone())
                    .unwrap_or_else(|| {
                        // Last resort: convert .rs to .wj
                        let mut wj_path = rust_location.file.clone();
                        wj_path.set_extension("wj");
                        wj_path
                    });

                Location {
                    file: wj_file,
                    line: rust_location.line,
                    column: rust_location.column,
                }
            });

        // Translate error message
        let message = self.translate_message(&rustc_diag.message);

        // Map additional spans
        let spans = rustc_diag
            .spans
            .iter()
            .filter_map(|span| self.map_span(span))
            .collect();

        // Extract help and notes from children
        let mut help = Vec::new();
        let mut notes = Vec::new();

        for child in &rustc_diag.children {
            match child.level.as_str() {
                "help" => help.push(child.message.clone()),
                "note" => notes.push(child.message.clone()),
                _ => {}
            }
        }

        // Map Rust error code to Windjammer error code
        let wj_code = rustc_diag
            .code
            .as_ref()
            .and_then(|c| {
                let registry = error_codes::get_registry();
                registry.map_rust_code(&c.code).map(|wj| wj.code.clone())
            });

        Some(WindjammerDiagnostic {
            message,
            level: match rustc_diag.level.as_str() {
                "error" => DiagnosticLevel::Error,
                "warning" => DiagnosticLevel::Warning,
                "note" => DiagnosticLevel::Note,
                "help" => DiagnosticLevel::Help,
                _ => DiagnosticLevel::Error,
            },
            location: wj_location,
            spans,
            code: wj_code.or_else(|| rustc_diag.code.as_ref().map(|c| c.code.clone())),
            help,
            notes,
        })
    }

    /// Map a rustc span to a Windjammer span
    fn map_span(&self, span: &RustcSpan) -> Option<DiagnosticSpan> {
        let rust_location = Location {
            file: PathBuf::from(&span.file_name),
            line: span.line_start,
            column: span.column_start,
        };

        let wj_location = self.source_map.map_rust_to_windjammer(&rust_location)?;

        Some(DiagnosticSpan {
            location: wj_location,
            label: span.label.clone(),
            is_primary: span.is_primary,
        })
    }

    /// Translate Rust error messages to Windjammer terminology
    fn translate_message(&self, rust_msg: &str) -> String {
        // Pattern matching for common Rust error patterns
        // Translate to Windjammer-friendly terminology

        // Type errors
        if rust_msg.contains("mismatched types") {
            return self.translate_type_mismatch(rust_msg);
        }

        if rust_msg.contains("cannot find type") {
            return self.translate_type_not_found(rust_msg);
        }

        if rust_msg.contains("cannot find value") || rust_msg.contains("cannot find function") {
            return self.translate_value_not_found(rust_msg);
        }

        // Ownership errors
        if rust_msg.contains("cannot move out of") {
            return "Ownership error: This value was already moved".to_string();
        }

        if rust_msg.contains("cannot borrow") && rust_msg.contains("as mutable") {
            return "Cannot modify: This value is not declared as mutable".to_string();
        }

        if rust_msg.contains("use of moved value") {
            return "Ownership error: This value was already used and cannot be used again"
                .to_string();
        }

        // Trait errors
        if rust_msg.contains("trait bounds were not satisfied") {
            return self.translate_trait_bounds(rust_msg);
        }

        if rust_msg.contains("the trait") && rust_msg.contains("is not implemented") {
            return self.translate_trait_not_implemented(rust_msg);
        }

        // Lifetime errors
        if rust_msg.contains("lifetime") {
            return self.translate_lifetime_error(rust_msg);
        }

        // Syntax errors
        if rust_msg.contains("expected") && rust_msg.contains("found") {
            return self.translate_syntax_error(rust_msg);
        }

        // Module/import errors
        if rust_msg.contains("unresolved import") {
            return "Import error: Module or item not found".to_string();
        }

        // Default: return original message
        rust_msg.to_string()
    }

    /// Translate type mismatch errors
    fn translate_type_mismatch(&self, rust_msg: &str) -> String {
        // Extract expected and found types
        if let (Some(expected), Some(found)) = (
            self.extract_between(rust_msg, "expected `", "`"),
            self.extract_between(rust_msg, "found `", "`"),
        ) {
            let expected_wj = self.rust_type_to_windjammer(&expected);
            let found_wj = self.rust_type_to_windjammer(&found);
            return format!(
                "Type mismatch: expected {}, found {}",
                expected_wj, found_wj
            );
        }

        "Type mismatch: The types don't match".to_string()
    }

    /// Translate type not found errors
    fn translate_type_not_found(&self, rust_msg: &str) -> String {
        if let Some(type_name) = self.extract_between(rust_msg, "cannot find type `", "`") {
            let wj_type = self.rust_type_to_windjammer(&type_name);
            return format!("Type not found: {}", wj_type);
        }

        "Type not found".to_string()
    }

    /// Translate value/function not found errors
    fn translate_value_not_found(&self, rust_msg: &str) -> String {
        if rust_msg.contains("cannot find function") {
            if let Some(func_name) = self.extract_between(rust_msg, "function `", "`") {
                return format!("Function not found: {}", func_name);
            }
            return "Function not found".to_string();
        }

        if let Some(value_name) = self.extract_between(rust_msg, "value `", "`") {
            return format!("Variable not found: {}", value_name);
        }

        "Value not found".to_string()
    }

    /// Translate trait bounds errors
    fn translate_trait_bounds(&self, _rust_msg: &str) -> String {
        "Trait constraint not satisfied: This type doesn't implement the required trait".to_string()
    }

    /// Translate trait not implemented errors
    fn translate_trait_not_implemented(&self, rust_msg: &str) -> String {
        if let Some(trait_name) = self.extract_between(rust_msg, "trait `", "`") {
            return format!("Missing trait implementation: {}", trait_name);
        }

        "Missing trait implementation".to_string()
    }

    /// Translate lifetime errors
    fn translate_lifetime_error(&self, _rust_msg: &str) -> String {
        "Lifetime error: The value doesn't live long enough".to_string()
    }

    /// Translate syntax errors
    fn translate_syntax_error(&self, rust_msg: &str) -> String {
        if let (Some(expected), Some(found)) = (
            self.extract_between(rust_msg, "expected ", ","),
            self.extract_between(rust_msg, "found ", "\n"),
        ) {
            return format!(
                "Syntax error: expected {}, found {}",
                expected.trim(),
                found.trim()
            );
        }

        "Syntax error".to_string()
    }

    /// Convert Rust type names to Windjammer type names
    fn rust_type_to_windjammer(&self, rust_type: &str) -> String {
        match rust_type {
            "i32" | "i64" | "isize" => "int",
            "u32" | "u64" | "usize" => "uint",
            "f32" | "f64" => "float",
            "&str" => "string",
            "String" => "string",
            "bool" => "bool",
            "()" => "void",
            _ => {
                // Handle references
                if rust_type.starts_with('&') {
                    return format!("&{}", self.rust_type_to_windjammer(&rust_type[1..]));
                }
                // Handle Option
                if rust_type.starts_with("Option<") {
                    if let Some(inner) = self.extract_between(rust_type, "Option<", ">") {
                        return format!("{}?", self.rust_type_to_windjammer(&inner));
                    }
                }
                // Handle Vec
                if rust_type.starts_with("Vec<") {
                    if let Some(inner) = self.extract_between(rust_type, "Vec<", ">") {
                        return format!("[{}]", self.rust_type_to_windjammer(&inner));
                    }
                }
                // Default: return as-is
                rust_type
            }
        }
        .to_string()
    }

    /// Extract text between two delimiters
    fn extract_between<'a>(&self, text: &'a str, start: &str, end: &str) -> Option<String> {
        let start_idx = text.find(start)? + start.len();
        let remaining = &text[start_idx..];
        let end_idx = remaining.find(end)?;
        Some(remaining[..end_idx].to_string())
    }
}

// ============================================================================
// PRETTY PRINTING
// ============================================================================

impl WindjammerDiagnostic {
    /// Check if this error is automatically fixable
    pub fn is_fixable(&self) -> bool {
        if let Some(code) = &self.code {
            matches!(code.as_str(), "E0384" | "E0308" | "E0425" | "E0596")
        } else {
            false
        }
    }
    
    /// Get the fix type for this error (if fixable)
    pub fn get_fix(&self) -> Option<crate::auto_fix::FixType> {
        use crate::auto_fix::FixType;
        
        if !self.is_fixable() {
            return None;
        }
        
        match self.code.as_ref()?.as_str() {
            "E0384" | "E0596" => {
                // Immutability error - suggest adding mut
                if let Some(var_name) = extract_variable_from_message(&self.message) {
                    Some(FixType::AddMut {
                        file: self.location.file.clone(),
                        line: self.location.line,
                        variable_name: var_name,
                    })
                } else {
                    None
                }
            }
            "E0308" => {
                // Type mismatch - suggest conversion
                if self.message.contains("expected int") && self.message.contains("found string") {
                    Some(FixType::AddParse {
                        file: self.location.file.clone(),
                        line: self.location.line,
                        column: self.location.column,
                        expression: "value".to_string(),
                    })
                } else if self.message.contains("expected String") && self.message.contains("found &str") {
                    Some(FixType::AddToString {
                        file: self.location.file.clone(),
                        line: self.location.line,
                        column: self.location.column,
                        expression: "value".to_string(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    
    /// Format this diagnostic for display (Rust-style pretty printing with colors)
    pub fn format(&self) -> String {
        use colored::*;
        
        let mut output = String::new();

        // Level and message (with colors!)
        let level_str = match self.level {
            DiagnosticLevel::Error => "error".red().bold(),
            DiagnosticLevel::Warning => "warning".yellow().bold(),
            DiagnosticLevel::Note => "note".blue().bold(),
            DiagnosticLevel::Help => "help".cyan().bold(),
        };

        if let Some(code) = &self.code {
            // Show Windjammer code prominently if it starts with WJ
            if code.starts_with("WJ") {
                output.push_str(&format!("{}[{}]: {}\n", level_str, code.cyan().bold(), self.message));
                output.push_str(&format!("  {} wj explain {}\n", "💡".yellow(), code));
            } else {
                output.push_str(&format!("{}[{}]: {}\n", level_str, code, self.message));
            }
        } else {
            output.push_str(&format!("{}: {}\n", level_str, self.message));
        }

        // Location (cyan for visibility)
        output.push_str(&format!(
            "  {} {}:{}:{}\n",
            "-->".cyan(),
            self.location.file.display(),
            self.location.line,
            self.location.column
        ));

        // Source code snippet
        if let Ok(snippet) = self.read_source_snippet() {
            output.push_str(&snippet);
        }

        // Help messages (cyan)
        for help_msg in &self.help {
            output.push_str(&format!("  = {}: {}\n", "help".cyan(), help_msg));
        }

        // Notes (blue)
        for note in &self.notes {
            output.push_str(&format!("  = {}: {}\n", "note".blue(), note));
        }

        // Contextual help (green for suggestions)
        if let Some(contextual_help) = self.get_contextual_help() {
            output.push_str(&format!("  = {}: {}\n", "suggestion".green().bold(), contextual_help));
        }

        output
    }

    /// Read and format the source code snippet for this error
    fn read_source_snippet(&self) -> Result<String, std::io::Error> {
        use colored::*;
        use std::fs;

        let source = fs::read_to_string(&self.location.file)?;
        let lines: Vec<&str> = source.lines().collect();

        let mut output = String::new();
        output.push_str(&format!("   {}\n", "|".cyan()));

        // Create syntax highlighter
        let highlighter = SyntaxHighlighter::new();

        // Show context: 2 lines before, the error line, and 2 lines after
        let start_line = self.location.line.saturating_sub(2);
        let end_line = (self.location.line + 2).min(lines.len());

        for line_num in start_line..=end_line {
            if line_num == 0 || line_num > lines.len() {
                continue;
            }

            let line = lines[line_num - 1];
            let is_error_line = line_num == self.location.line;

            // Apply syntax highlighting to the line
            let highlighted_line = highlighter.highlight_line(line);

            if is_error_line {
                // Error line with pointer (red for errors, yellow for warnings)
                let pointer_color = match self.level {
                    DiagnosticLevel::Error => "^".red().bold(),
                    DiagnosticLevel::Warning => "^".yellow().bold(),
                    _ => "^".cyan(),
                };
                
                output.push_str(&format!("{:>4} {} {}\n", 
                    line_num.to_string().cyan(), 
                    "|".cyan(), 
                    highlighted_line
                ));
                output.push_str(&format!("   {} {}{}\n", 
                    "|".cyan(),
                    " ".repeat(self.location.column.saturating_sub(1)),
                    pointer_color
                ));
            } else {
                // Context line with syntax highlighting
                output.push_str(&format!("{:>4} {} {}\n", 
                    line_num.to_string().cyan(), 
                    "|".cyan(), 
                    highlighted_line
                ));
            }
        }

        output.push_str(&format!("   {}\n", "|".cyan()));
        Ok(output)
    }

    /// Get Windjammer-specific contextual help based on the error message
    fn get_contextual_help(&self) -> Option<String> {
        let msg = &self.message.to_lowercase();

        // Type mismatch suggestions
        if msg.contains("type mismatch") {
            if msg.contains("expected int") && msg.contains("found string") {
                return Some(
                    "Use .parse() to convert a string to an integer, e.g., \"42\".parse()"
                        .to_string(),
                );
            }
            if msg.contains("expected string") && msg.contains("found int") {
                return Some(
                    "Use .to_string() to convert an integer to a string, e.g., 42.to_string()"
                        .to_string(),
                );
            }
            if msg.contains("expected &") {
                return Some("Add & before the value to create a reference".to_string());
            }
        }

        // Function not found
        if msg.contains("function not found") {
            return Some(
                "Check the function name spelling and ensure the module is imported".to_string(),
            );
        }

        // Variable not found
        if msg.contains("variable not found") {
            return Some(
                "Check the variable name spelling and ensure it's declared before use".to_string(),
            );
        }

        // Ownership errors
        if msg.contains("ownership error") {
            return Some("In Windjammer, values can only be used once unless they implement Copy. Consider using references (&) or cloning (.clone())".to_string());
        }

        // Mutability errors
        if msg.contains("cannot modify") {
            return Some("Declare the variable as mutable: let mut x = ...".to_string());
        }

        // Import errors
        if msg.contains("import error") {
            return Some("Use 'use module::item' to import, or check if the module exists in your project or stdlib".to_string());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rustc_json() {
        let json = r#"{"message":"mismatched types","level":"error","spans":[{"file_name":"test.rs","line_start":10,"line_end":10,"column_start":5,"column_end":10,"is_primary":true,"label":"expected i32, found &str","text":null}],"code":{"code":"E0308"},"children":[],"rendered":null}"#;

        let diag: RustcDiagnostic = serde_json::from_str(json).unwrap();
        assert_eq!(diag.message, "mismatched types");
        assert_eq!(diag.level, "error");
        assert_eq!(diag.spans.len(), 1);
        assert_eq!(diag.spans[0].line_start, 10);
    }

    #[test]
    fn test_diagnostic_format() {
        let diag = WindjammerDiagnostic {
            message: "Type mismatch".to_string(),
            level: DiagnosticLevel::Error,
            location: Location {
                file: PathBuf::from("test.wj"),
                line: 10,
                column: 5,
            },
            spans: vec![],
            code: Some("E0308".to_string()),
            help: vec!["Try using .parse()".to_string()],
            notes: vec![],
        };

        let formatted = diag.format();
        assert!(formatted.contains("error[E0308]"));
        assert!(formatted.contains("test.wj:10:5"));
        assert!(formatted.contains("help: Try using .parse()"));
    }

    #[test]
    fn test_rust_type_to_windjammer() {
        let mapper = ErrorMapper::new(SourceMap::new());

        assert_eq!(mapper.rust_type_to_windjammer("i32"), "int");
        assert_eq!(mapper.rust_type_to_windjammer("i64"), "int");
        assert_eq!(mapper.rust_type_to_windjammer("&str"), "string");
        assert_eq!(mapper.rust_type_to_windjammer("String"), "string");
        assert_eq!(mapper.rust_type_to_windjammer("bool"), "bool");
        assert_eq!(mapper.rust_type_to_windjammer("f64"), "float");
        assert_eq!(mapper.rust_type_to_windjammer("()"), "void");
    }

    #[test]
    fn test_rust_type_to_windjammer_complex() {
        let mapper = ErrorMapper::new(SourceMap::new());

        assert_eq!(mapper.rust_type_to_windjammer("&i32"), "&int");
        assert_eq!(mapper.rust_type_to_windjammer("Vec<i32>"), "[int]");
        assert_eq!(mapper.rust_type_to_windjammer("Option<String>"), "string?");
    }

    #[test]
    fn test_translate_type_mismatch() {
        let mapper = ErrorMapper::new(SourceMap::new());

        let rust_msg = "mismatched types: expected `i32`, found `&str`";
        let translated = mapper.translate_message(rust_msg);
        assert!(translated.contains("Type mismatch"));
        assert!(translated.contains("int"));
        assert!(translated.contains("string"));
    }

    #[test]
    fn test_translate_function_not_found() {
        let mapper = ErrorMapper::new(SourceMap::new());

        let rust_msg = "cannot find function `foo` in this scope";
        let translated = mapper.translate_message(rust_msg);
        assert!(translated.contains("Function not found"));
        assert!(translated.contains("foo"));
    }

    #[test]
    fn test_translate_ownership_error() {
        let mapper = ErrorMapper::new(SourceMap::new());

        let rust_msg = "use of moved value: `x`";
        let translated = mapper.translate_message(rust_msg);
        assert!(translated.contains("Ownership error"));
    }

    #[test]
    fn test_contextual_help_type_mismatch() {
        let diag = WindjammerDiagnostic {
            message: "Type mismatch: expected int, found string".to_string(),
            level: DiagnosticLevel::Error,
            location: Location {
                file: PathBuf::from("test.wj"),
                line: 10,
                column: 5,
            },
            spans: vec![],
            code: None,
            help: vec![],
            notes: vec![],
        };

        let help = diag.get_contextual_help();
        assert!(help.is_some());
        assert!(help.unwrap().contains(".parse()"));
    }

    #[test]
    fn test_contextual_help_mutability() {
        let diag = WindjammerDiagnostic {
            message: "Cannot modify: This value is not declared as mutable".to_string(),
            level: DiagnosticLevel::Error,
            location: Location {
                file: PathBuf::from("test.wj"),
                line: 10,
                column: 5,
            },
            spans: vec![],
            code: None,
            help: vec![],
            notes: vec![],
        };

        let help = diag.get_contextual_help();
        assert!(help.is_some());
        assert!(help.unwrap().contains("let mut"));
    }

    #[test]
    fn test_extract_between() {
        let mapper = ErrorMapper::new(SourceMap::new());

        let text = "expected `i32`, found `&str`";
        let expected = mapper.extract_between(text, "expected `", "`");
        assert_eq!(expected, Some("i32".to_string()));

        let found = mapper.extract_between(text, "found `", "`");
        assert_eq!(found, Some("&str".to_string()));
    }
}


/// Extract variable name from error message
fn extract_variable_from_message(msg: &str) -> Option<String> {
    // Look for patterns like "cannot assign twice to immutable variable `x`"
    if let Some(start) = msg.find("`") {
        if let Some(end) = msg[start + 1..].find("`") {
            return Some(msg[start + 1..start + 1 + end].to_string());
        }
    }
    None
}
