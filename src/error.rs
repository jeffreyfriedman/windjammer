//! Windjammer Compiler Error Types
//!
//! Provides rich error reporting with source locations, code snippets,
//! and helpful suggestions - inspired by Rust's excellent error messages.

use std::fmt;

/// Source location for an error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(file: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
}

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLevel {
    Error,
    Warning,
    Note,
}

/// A suggestion for fixing an error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub message: String,
    pub replacement: Option<String>,
}

impl Suggestion {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            replacement: None,
        }
    }

    pub fn with_replacement(message: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            replacement: Some(replacement.into()),
        }
    }
}

/// A compiler error with rich context
#[derive(Debug, Clone)]
pub struct CompileError {
    pub level: ErrorLevel,
    pub message: String,
    pub location: SourceLocation,
    pub code_snippet: Option<String>,
    pub suggestions: Vec<Suggestion>,
}

impl CompileError {
    /// Create a new error
    pub fn new(message: impl Into<String>, location: SourceLocation) -> Self {
        Self {
            level: ErrorLevel::Error,
            message: message.into(),
            location,
            code_snippet: None,
            suggestions: Vec::new(),
        }
    }

    /// Create a parse error
    pub fn parse_error(
        message: impl Into<String>,
        file: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        Self::new(message, SourceLocation::new(file, line, column))
    }

    /// Add a code snippet to show in the error
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.code_snippet = Some(snippet.into());
        self
    }

    /// Add a suggestion for fixing the error
    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add a simple suggestion message
    pub fn suggest(mut self, message: impl Into<String>) -> Self {
        self.suggestions.push(Suggestion::new(message));
        self
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Error header with location
        let level_str = match self.level {
            ErrorLevel::Error => "error",
            ErrorLevel::Warning => "warning",
            ErrorLevel::Note => "note",
        };

        writeln!(f, "{}: {}", level_str, self.message)?;

        writeln!(
            f,
            "  --> {}:{}:{}",
            self.location.file, self.location.line, self.location.column
        )?;

        // Code snippet if available
        if let Some(ref snippet) = self.code_snippet {
            writeln!(f, "   |")?;
            for (i, line) in snippet.lines().enumerate() {
                let line_num = self.location.line + i;
                writeln!(f, "{:3} | {}", line_num, line)?;
            }
            writeln!(f, "   |")?;

            // Add caret pointing to the error location
            if self.location.column > 0 {
                let spaces = " ".repeat(self.location.column - 1);
                writeln!(f, "   | {}^", spaces)?;
            }
        }

        // Suggestions
        for suggestion in &self.suggestions {
            writeln!(f, "   = help: {}", suggestion.message)?;
            if let Some(ref replacement) = suggestion.replacement {
                writeln!(f, "   = suggestion: {}", replacement)?;
            }
        }

        Ok(())
    }
}

impl std::error::Error for CompileError {}

/// Result type for compiler operations
pub type CompileResult<T> = Result<T, CompileError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = CompileError::parse_error("Expected ']', got '}'", "test.wj", 3, 15)
            .with_snippet("    let x = [1, 2, 3")
            .suggest("Add ']' before the newline");

        let output = format!("{}", error);
        assert!(output.contains("error: Expected ']', got '}'"));
        assert!(output.contains("test.wj:3:15"));
        assert!(output.contains("help: Add ']' before the newline"));
    }
}
