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
    /// A sink was called without the capability (ADR-0021). Not catchable; exit status 126.
    CapabilityDenied,
    /// A resource limit of a sandboxed run was exceeded (`--limits`). Not catchable; exit status 124.
    Limit,
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
                out.push_str(&format!("thread 'main' panicked at {}:\n{}", at(s), self.message))
            }
            (ErrorKind::Panic, None) => {
                out.push_str(&format!("thread 'main' panicked:\n{}", self.message))
            }
            (ErrorKind::CapabilityDenied, s) => {
                out.push_str(&format!("error: {}{}", self.message, s.as_ref().map(|s| format!("\n  --> {}", s)).unwrap_or_default()))
            }
            (ErrorKind::Limit, s) => {
                out.push_str(&format!("error: {}{}", self.message, s.as_ref().map(|s| format!("\n  --> {}", s)).unwrap_or_default()))
            }
            (ErrorKind::Unsupported | ErrorKind::Semantic, s) if crate::error::runtime_text::classify(&self.message).is_some() => {
                // Presentation only: stable code + a one-line "what to write" (diagnostics::runtime_text).
                let c = crate::error::runtime_text::classify(&self.message).unwrap();
                out.push_str(&format!(
                    "error: {} [{}]{}\n  = help: {}",
                    c.message,
                    c.code,
                    s.as_ref().map(|s| format!("\n  --> {}", s)).unwrap_or_default(),
                    c.help
                ));
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
            // After the Rust-shaped head (which tools compare with compiled output, up to this line): the
            // failing source line with a caret, and the chain of Windjammer calls on one line.
            out.push_str("\nstack backtrace:");
            if let Some(excerpt) = self.span.as_ref().and_then(source_excerpt) {
                out.push_str(&excerpt);
            }
            out.push_str(&format!("\n  {}", call_chain(&self.backtrace)));
        }
        out
    }
}

/// `file` relative to the current directory when it lies below it (shorter for people and agents alike).
pub(crate) fn short_path(file: &str) -> String {
    let cwd = std::env::current_dir().ok().map(|d| d.to_string_lossy().to_string() + "/");
    match cwd.as_deref().and_then(|c| file.strip_prefix(c)) {
        Some(rel) => rel.to_string(),
        None => file.to_string(),
    }
}

/// `file:line:col` with a shortened path.
fn at(s: &Span) -> String {
    format!("{}:{}:{}", short_path(&s.file), s.line, s.column)
}

/// The source line of `s` with a caret under its column, as `wj check` shows diagnostics.
fn source_excerpt(s: &Span) -> Option<String> {
    let text = std::fs::read_to_string(&s.file).ok()?;
    let line = text.lines().nth(s.line.checked_sub(1)?)?;
    let n = s.line.to_string();
    let pad = " ".repeat(n.len());
    let caret = " ".repeat(s.column.saturating_sub(1).min(line.chars().count()));
    Some(format!("\n {pad}--> {}\n {n} | {line}\n {pad} | {caret}^", at(s)))
}

/// `in `f`, called from `g` at file:line:col, called from `main` at ..`: the Windjammer frames, innermost first.
fn call_chain(frames: &[Frame]) -> String {
    let mut out = format!("in `{}`", frames[0].function);
    for (i, fr) in frames.iter().enumerate().skip(1) {
        out.push_str(&format!(", called from `{}`", fr.function));
        if let Some(site) = &frames[i - 1].call_site {
            out.push_str(&format!(" at {}", at(site)));
        }
    }
    out
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
    /// Boxed so `Result<Value, Ctl>` stays small (it is returned by every evaluated node).
    Break(Box<(Option<String>, Value)>),
    /// `continue`, `continue 'label`.
    Continue(Box<Option<String>>),
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
