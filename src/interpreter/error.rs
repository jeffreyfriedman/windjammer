//! Runtime errors and non-local control flow.

use super::value::Value;
use std::fmt;

/// Where a runtime error happened (file:line:col of the nearest located AST node).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// Rust-equivalent panic (overflow, index out of bounds, unwrap on None, explicit `panic`).
    Panic,
    /// The program uses something the interpreter does not implement. Never silently wrong.
    Unsupported,
    /// Name resolution / type errors the compiler would have rejected.
    Semantic,
    /// `std::process::exit(code)`
    Exit(i32),
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub function: String,
    pub call_site: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Option<Span>,
    pub backtrace: Vec<Frame>,
}

impl RuntimeError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        RuntimeError {
            kind,
            message: message.into(),
            span: None,
            backtrace: Vec::new(),
        }
    }

    /// Rust-style rendering: `panicked at file:line:col:` + message + backtrace.
    pub fn render(&self) -> String {
        let mut out = String::new();
        match (&self.kind, &self.span) {
            (ErrorKind::Panic, Some(s)) => {
                out.push_str(&format!("thread 'main' panicked at {}:\n{}", s, self.message))
            }
            (ErrorKind::Panic, None) => {
                out.push_str(&format!("thread 'main' panicked:\n{}", self.message))
            }
            (ErrorKind::Unsupported, s) => {
                out.push_str(&format!(
                    "error: unsupported in the interpreter: {}{}\n  = help: run it compiled with `wj run --compiled`",
                    self.message,
                    s.as_ref().map(|s| format!("\n  --> {}", s)).unwrap_or_default()
                ));
            }
            (_, Some(s)) => out.push_str(&format!("error: {}\n  --> {}", self.message, s)),
            (_, None) => out.push_str(&format!("error: {}", self.message)),
        }
        if !self.backtrace.is_empty() {
            out.push_str("\nstack backtrace:");
            for (i, fr) in self.backtrace.iter().enumerate() {
                out.push_str(&format!("\n  {}: {}", i, fr.function));
                if let Some(s) = &fr.call_site {
                    out.push_str(&format!("\n        at {}", s));
                }
            }
        }
        out
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.span {
            Some(s) => write!(f, "{} ({})", self.message, s),
            None => write!(f, "{}", self.message),
        }
    }
}

/// Non-local exits threaded through `Result<Value, Ctl>`.
#[derive(Debug)]
pub enum Ctl {
    /// `break`, `break 'label`, `break value`. `value` is `Unit` for a plain break.
    Break { label: Option<String>, value: Value },
    /// `continue`, `continue 'label`.
    Continue { label: Option<String> },
    Return(Value),
    Err(Box<RuntimeError>),
}

impl From<RuntimeError> for Ctl {
    fn from(e: RuntimeError) -> Ctl {
        Ctl::Err(Box::new(e))
    }
}

pub type R<T = Value> = Result<T, Ctl>;

pub fn panic_ctl(msg: impl Into<String>) -> Ctl {
    RuntimeError::new(ErrorKind::Panic, msg).into()
}
pub fn unsupported(msg: impl Into<String>) -> Ctl {
    RuntimeError::new(ErrorKind::Unsupported, msg).into()
}
pub fn semantic(msg: impl Into<String>) -> Ctl {
    RuntimeError::new(ErrorKind::Semantic, msg).into()
}
