//! Parser Error Recovery
//!
//! This module provides comprehensive error recovery for the Windjammer parser.
//! It allows the parser to continue parsing after encountering errors, collecting
//! multiple errors in a single pass rather than failing on the first error.
//!
//! ## Features
//!
//! 1. **Error Collection** - Accumulate multiple parse errors
//! 2. **Smart Recovery** - Skip to synchronization points (semicolons, braces, keywords)
//! 3. **Partial AST** - Return valid portions of the AST even with errors
//! 4. **Helpful Messages** - Context-aware error messages with suggestions
//!
//! ## Recovery Strategies
//!
//! - **Statement-level**: Skip to next semicolon or closing brace
//! - **Expression-level**: Skip to next operator or delimiter
//! - **Item-level**: Skip to next top-level keyword (fn, struct, impl, etc.)
//!
//! ## Example
//!
//! ```ignore
//! // Input with errors:
//! fn broken(x: ) {  // Missing type
//!     let y = ;     // Missing value
//!     x + 1
//! }
//!
//! fn valid() -> int {  // This function is fine
//!     42
//! }
//!
//! // Parser recovers and reports both errors, still parses valid()
//! ```

use crate::lexer::Token;
use std::fmt;

/// Parse error with context
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Token where error occurred
    pub token: Option<Token>,
    /// Position in token stream
    pub position: usize,
    /// Error category
    pub category: ErrorCategory,
    /// Suggested fix (if available)
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Unexpected token
    UnexpectedToken,
    /// Missing expected token
    MissingToken,
    /// Invalid syntax
    InvalidSyntax,
    /// Type error
    TypeError,
    /// Name resolution error
    NameError,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at position {}: {}",
            self.position, self.message
        )?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, "\n  Suggestion: {}", suggestion)?;
        }
        Ok(())
    }
}

impl ParseError {
    pub fn new(message: String, position: usize) -> Self {
        ParseError {
            message,
            token: None,
            position,
            category: ErrorCategory::InvalidSyntax,
            suggestion: None,
        }
    }

    pub fn with_token(mut self, token: Token) -> Self {
        self.token = Some(token);
        self
    }

    pub fn with_category(mut self, category: ErrorCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

/// Recovery point - where to synchronize after an error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPoint {
    /// Semicolon (statement boundary)
    Semicolon,
    /// Closing brace (block boundary)
    CloseBrace,
    /// Top-level keyword (fn, struct, impl, etc.)
    TopLevelKeyword,
    /// Any of the above
    Any,
}

/// Check if a token is a recovery point
pub fn is_recovery_point(token: &Token, point: RecoveryPoint) -> bool {
    match point {
        RecoveryPoint::Semicolon => matches!(token, Token::Semicolon),
        RecoveryPoint::CloseBrace => matches!(token, Token::RBrace),
        RecoveryPoint::TopLevelKeyword => matches!(
            token,
            Token::Fn
                | Token::Struct
                | Token::Enum
                | Token::Trait
                | Token::Impl
                | Token::Const
                | Token::Static
                | Token::Use
        ),
        RecoveryPoint::Any => {
            is_recovery_point(token, RecoveryPoint::Semicolon)
                || is_recovery_point(token, RecoveryPoint::CloseBrace)
                || is_recovery_point(token, RecoveryPoint::TopLevelKeyword)
        }
    }
}

/// Generate a helpful error message for unexpected token
pub fn unexpected_token_message(expected: &str, found: &Token) -> String {
    format!("Expected {}, found {:?}", expected, found)
}

/// Generate a helpful error message for missing token
pub fn missing_token_message(expected: &str) -> String {
    format!("Expected {}, but reached end of input", expected)
}

/// Suggest a fix for common mistakes
pub fn suggest_fix(error_context: &str) -> Option<String> {
    match error_context {
        "missing_type" => Some("Add a type annotation (e.g., ': int', ': string')".to_string()),
        "missing_value" => Some("Add an expression after '='".to_string()),
        "missing_semicolon" => Some("Add a semicolon ';' at the end of the statement".to_string()),
        "missing_brace" => Some("Add a closing brace '}'".to_string()),
        "missing_paren" => Some("Add a closing parenthesis ')'".to_string()),
        "missing_bracket" => Some("Add a closing bracket ']'".to_string()),
        _ => None,
    }
}

/// Parser result that can contain errors
pub type ParseResult<T> = Result<T, Vec<ParseError>>;

/// Wrapper for partial results with accumulated errors
#[derive(Debug, Clone)]
pub struct PartialResult<T> {
    /// The parsed value (may be incomplete or placeholder)
    pub value: T,
    /// Accumulated errors during parsing
    pub errors: Vec<ParseError>,
}

impl<T> PartialResult<T> {
    pub fn ok(value: T) -> Self {
        PartialResult {
            value,
            errors: Vec::new(),
        }
    }

    pub fn with_error(value: T, error: ParseError) -> Self {
        PartialResult {
            value,
            errors: vec![error],
        }
    }

    pub fn with_errors(value: T, errors: Vec<ParseError>) -> Self {
        PartialResult { value, errors }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn into_result(self) -> Result<T, Vec<ParseError>> {
        if self.errors.is_empty() {
            Ok(self.value)
        } else {
            Err(self.errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_display() {
        let error =
            ParseError::new("test error".to_string(), 10).with_suggestion("try this".to_string());
        let display = format!("{}", error);
        assert!(display.contains("test error"));
        assert!(display.contains("try this"));
    }

    #[test]
    fn test_recovery_points() {
        assert!(is_recovery_point(
            &Token::Semicolon,
            RecoveryPoint::Semicolon
        ));
        assert!(is_recovery_point(&Token::RBrace, RecoveryPoint::CloseBrace));
        assert!(is_recovery_point(
            &Token::Fn,
            RecoveryPoint::TopLevelKeyword
        ));
        assert!(is_recovery_point(&Token::Semicolon, RecoveryPoint::Any));
    }

    #[test]
    fn test_partial_result() {
        let ok_result = PartialResult::ok(42);
        assert!(!ok_result.has_errors());

        let err_result = PartialResult::with_error(42, ParseError::new("error".to_string(), 0));
        assert!(err_result.has_errors());
        assert_eq!(err_result.errors.len(), 1);
    }
}
