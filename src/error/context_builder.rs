//! Enhanced compiler error context with source-line formatting.
//!
//! Provides Rust-style error output with caret pointing to the error location
//! and optional help/note messages.

/// Context-aware error display with source location and suggestions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorContext {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub source_line: String,
    pub error_type: String,
    pub message: String,
    pub help: Option<String>,
    pub note: Option<String>,
}

impl ErrorContext {
    /// Format the error in Rust-style output with source line and caret.
    pub fn format(&self) -> String {
        let mut output = format!(
            "error[{}]: {}\n  --> {}:{}:{}\n",
            self.error_type, self.message, self.file, self.line, self.column
        );

        // Add source line with caret
        output.push_str(&format!("   |\n{:3} | {}\n", self.line, self.source_line));
        let caret_spaces = if self.column > 0 {
            " ".repeat(self.column - 1)
        } else {
            String::new()
        };
        output.push_str(&format!("   | {}^\n", caret_spaces));

        if let Some(help) = &self.help {
            output.push_str(&format!("   |\nhelp: {}\n", help));
        }

        if let Some(note) = &self.note {
            output.push_str(&format!("note: {}\n", note));
        }

        output
    }

    /// Builder: set the error type (e.g., "E0425").
    pub fn with_error_type(mut self, error_type: impl Into<String>) -> Self {
        self.error_type = error_type.into();
        self
    }

    /// Builder: set the help message.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Builder: set the note message.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_formatting() {
        let ctx = ErrorContext {
            file: "game.wj".to_string(),
            line: 42,
            column: 15,
            source_line: "    let x = undefined_var".to_string(),
            error_type: "E0425".to_string(),
            message: "cannot find value `undefined_var`".to_string(),
            help: Some("did you mean `defined_var`?".to_string()),
            note: None,
        };

        let formatted = ctx.format();
        assert!(formatted.contains("game.wj:42:15"));
        assert!(formatted.contains("undefined_var"));
        assert!(formatted.contains("help: did you mean"));
    }

    #[test]
    fn test_error_context_includes_error_type() {
        let ctx = ErrorContext {
            file: "test.wj".to_string(),
            line: 1,
            column: 5,
            source_line: "  x = y".to_string(),
            error_type: "E0308".to_string(),
            message: "type mismatch".to_string(),
            help: None,
            note: None,
        };

        let formatted = ctx.format();
        assert!(formatted.contains("error[E0308]"));
        assert!(formatted.contains("type mismatch"));
    }

    #[test]
    fn test_error_context_includes_note() {
        let ctx = ErrorContext {
            file: "game.wj".to_string(),
            line: 10,
            column: 1,
            source_line: "fn foo()".to_string(),
            error_type: "E0404".to_string(),
            message: "expected type".to_string(),
            help: Some("add type annotation".to_string()),
            note: Some("required by trait bound".to_string()),
        };

        let formatted = ctx.format();
        assert!(formatted.contains("note: required by trait bound"));
    }

    #[test]
    fn test_error_context_caret_position() {
        let ctx = ErrorContext {
            file: "a.wj".to_string(),
            line: 42,
            column: 10,
            source_line: "    let x = 5".to_string(),
            error_type: "E".to_string(),
            message: "test".to_string(),
            help: None,
            note: None,
        };

        let formatted = ctx.format();
        // Caret should be at column 10 (9 spaces before ^)
        assert!(formatted.contains("         ^"));
    }

    #[test]
    fn test_error_context_column_zero() {
        let ctx = ErrorContext {
            file: "a.wj".to_string(),
            line: 1,
            column: 0,
            source_line: "fn main()".to_string(),
            error_type: "E".to_string(),
            message: "test".to_string(),
            help: None,
            note: None,
        };

        let formatted = ctx.format();
        // Should not panic; caret at start
        assert!(formatted.contains("   |"));
        assert!(formatted.contains("^"));
    }

    #[test]
    fn test_error_context_builder_methods() {
        let ctx = ErrorContext {
            file: "game.wj".to_string(),
            line: 1,
            column: 1,
            source_line: "x".to_string(),
            error_type: "E0425".to_string(),
            message: "not found".to_string(),
            help: None,
            note: None,
        };

        let ctx = ctx.with_help("declare it first").with_note("in scope");
        assert_eq!(ctx.help.as_deref(), Some("declare it first"));
        assert_eq!(ctx.note.as_deref(), Some("in scope"));
    }
}
