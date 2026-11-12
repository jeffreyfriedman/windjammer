//! A runtime error as a diagnostics-schema report (`--error-format json`, ADR-0036).
//!
//! The shape is the `wj check --json` report (docs/diagnostics-schema.json, `schema_version` 2) with `command`
//! `run`: one diagnostic whose `primary_span` is the failing expression, plus the runtime-only members `stack`
//! (frames, innermost first, each with the span it is executing) and `locals` (bindings of the failing frame).

use super::error::{ErrorKind, RuntimeError, Span};
use serde_json::{json, Value};

/// A panic of the program (`unwrap`, index, overflow, division, `panic`, assertions).
pub const CODE_PANIC: &str = "WJ0170";
/// `main` returned `Err`.
pub const CODE_MAIN_ERR: &str = "WJ0171";
/// Any other runtime failure (refusals, limits, capability denials).
pub const CODE_OTHER: &str = "WJ0172";

/// The schema span of a point `file:line:col`, with byte offsets when the file can be read.
fn span_json(s: &Span) -> Value {
    let text = std::fs::read_to_string(&s.file).unwrap_or_default();
    let line_start: usize = text.split_inclusive('\n').take(s.line.saturating_sub(1)).map(str::len).sum();
    let line = text.lines().nth(s.line.saturating_sub(1)).unwrap_or("");
    let col0 = s.column.saturating_sub(1);
    let byte_in_line = line.char_indices().nth(col0).map(|(b, _)| b).unwrap_or(line.len());
    let width = line[byte_in_line..].chars().next().map(char::len_utf8).unwrap_or(0);
    let start = line_start + byte_in_line;
    json!({
        "file": super::error::short_path(&s.file),
        "byte_start": start,
        "byte_end": start + width,
        "line_start": s.line.max(1),
        "col_start": s.column.max(1),
        "line_end": s.line.max(1),
        "col_end": s.column.max(1) + usize::from(width > 0),
    })
}

/// One diagnostic of the schema (`docs/diagnostics-schema.json`): the fields every diagnostic has.
fn diagnostic(code: &str, message: &str) -> Value {
    json!({
        "code": code,
        "severity": "error",
        "category": "runtime",
        "origin": "runtime",
        "message": message,
        "primary_span": Value::Null,
        "docs": { "explain": format!("wj explain {code}"), "url": format!("https://windjammer.dev/errors/{code}") },
        "maturity": "proven",
    })
}

fn finish(mut diag: Value, span: Option<&Span>, stack: Vec<Value>, locals: &RuntimeError, exit: i32) -> Value {
    diag["primary_span"] = span.map(span_json).unwrap_or(Value::Null);
    diag["stack"] = Value::Array(stack);
    diag["locals"] = Value::Array(
        locals.locals.iter().map(|l| json!({ "name": l.name, "value": l.value, "redacted": l.redacted })).collect(),
    );
    if locals.locals_omitted > 0 {
        diag["locals_omitted"] = json!(locals.locals_omitted);
    }
    json!({
        "schema_version": 2,
        "windjammer_version": env!("CARGO_PKG_VERSION"),
        "command": "run",
        "success": false,
        "exit_code": exit,
        "summary": { "errors": 1, "warnings": 0, "files_checked": 0, "files_cached": 0, "elapsed_ms": 0 },
        "diagnostics": [diag],
    })
}

/// The report for a failed run, as one line of JSON.
pub fn runtime_error_json(e: &RuntimeError, exit: i32) -> String {
    let code = if e.kind == ErrorKind::Panic { CODE_PANIC } else { CODE_OTHER };
    let mut d = diagnostic(code, &e.message);
    d["help"] = json!(["the bindings of the failing frame are in `locals`; `--no-locals` leaves them out"]);
    let stack = e
        .frame_sites()
        .into_iter()
        .map(|(function, site)| json!({ "function": function, "span": site.as_ref().map(span_json) }))
        .collect();
    finish(d, e.span.as_ref(), stack, e, exit).to_string()
}

/// The report for `main` returning `Err(value)`; `text` is the value's Debug form. `site` is where `main` produced
/// the `Err` (the `Err(..)`, `return` or `?` that ran last) with the bindings of its frame, when it was noted.
pub(crate) fn main_err_json(text: &str, site: Option<&super::failure_locals::MainErrSite>) -> String {
    let d = diagnostic(CODE_MAIN_ERR, &format!("main returned an error: {text}"));
    let span = site.and_then(|s| s.span.as_ref());
    let stack = vec![json!({ "function": "main", "span": span.map(span_json) })];
    let mut e = RuntimeError::new(ErrorKind::Exit(1), "");
    if let Some(s) = site {
        e.locals = s.locals.clone();
        e.locals_omitted = s.omitted;
    }
    finish(d, span, stack, &e, 1).to_string()
}
