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

/// One live binding of the failing frame, rendered when the error was raised (never on the happy path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub name: String,
    /// Truncated Debug-style text, or `<redacted>`.
    pub value: String,
    pub redacted: bool,
}

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Option<Span>,
    pub backtrace: Vec<Frame>,
    /// Bindings of the innermost user frame at the failure (`failure_locals.rs`); empty with `--no-locals`.
    pub locals: Vec<Local>,
    /// Bindings that did not fit under the cap.
    pub locals_omitted: usize,
}

impl RuntimeError {
    /// Frames innermost first, each with the place it is executing: the error span for the innermost, the
    /// call site recorded by the callee for every outer frame.
    pub fn frame_sites(&self) -> Vec<(String, Option<Span>)> {
        self.backtrace
            .iter()
            .enumerate()
            .map(|(i, fr)| {
                let site = if i == 0 { self.span.clone() } else { self.backtrace[i - 1].call_site.clone() };
                (fr.function.clone(), site)
            })
            .collect()
    }

    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        RuntimeError {
            kind,
            message: message.into(),
            span: None,
            backtrace: Vec::new(),
            locals: Vec::new(),
            locals_omitted: 0,
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
            // failing source line with a caret, the Windjammer frames (innermost first) and the live bindings
            // of the failing frame. Interpreter-only detail: it never reaches stdout or the exit status.
            out.push_str("\nstack backtrace:");
            if let Some(excerpt) = self.span.as_ref().and_then(source_excerpt) {
                out.push_str(&excerpt);
            }
            for (i, (name, at_span)) in self.frame_sites().iter().enumerate() {
                match at_span {
                    Some(s) => out.push_str(&format!("\n  {i}: `{name}` at {}", at(s))),
                    None => out.push_str(&format!("\n  {i}: `{name}`")),
                }
            }
            if !self.locals.is_empty() {
                out.push_str(&format!("\nlocals in `{}`:", self.backtrace[0].function));
                for l in &self.locals {
                    out.push_str(&format!("\n  {} = {}", l.name, l.value));
                }
                if self.locals_omitted > 0 {
                    out.push_str(&format!("\n  .. and {} more", self.locals_omitted));
                }
            }
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
pub(crate) fn at(s: &Span) -> String {
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
