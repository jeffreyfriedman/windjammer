//! Error display component

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub severity: ErrorSeverity,
    pub code: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

impl CompilerError {
    pub fn new(message: String, line: usize, column: usize) -> Self {
        Self {
            message,
            line,
            column,
            severity: ErrorSeverity::Error,
            code: None,
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }
}

pub struct ErrorDisplay {
    pub errors: Vec<CompilerError>,
}

impl ErrorDisplay {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn add_error(&mut self, error: CompilerError) {
        self.errors.push(error);
    }

    pub fn clear(&mut self) {
        self.errors.clear();
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl Default for ErrorDisplay {
    fn default() -> Self {
        Self::new()
    }
}
