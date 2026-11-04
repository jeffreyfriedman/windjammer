//! Uses after a move that rustc would reject (E0382, reported as WJ0007), found by `wj check` and therefore
//! before an interpreted run.
//!
//! The interpreter copies values, so a program that uses a value after moving it runs interpreted and
//! fails only when built. The prediction reads the Rust the compiler generates for the file (in-process,
//! no cargo; `ir::explain::moves`), so it follows the compiler's own decisions about receivers, formals,
//! clones and `Copy` types (ADR-0030). It runs in the per-file analysis phase, so the check cache keeps its
//! result. A file the generator cannot handle on its own is skipped.

use crate::diagnostics::{Diagnostic, Span};
use crate::ir::explain::moves::{Move, MoveKind};
use std::path::Path;

/// WJ0007 diagnostics for the single file at `path` (tests and tools).
pub fn predicted_moves(path: &Path) -> Vec<Diagnostic> {
    let Ok(source) = std::fs::read_to_string(path) else { return Vec::new() };
    let file = path.to_string_lossy().to_string();
    let tokens = crate::lexer::Lexer::new(&source).tokenize_with_locations();
    let Ok(program) = crate::parser::Parser::new_with_source(tokens, file.clone(), source.clone()).parse() else { return Vec::new() };
    predict(&program, &source, &file)
}

/// WJ0007 diagnostics for one parsed file.
pub fn predict(program: &crate::parser::Program<'_>, source: &str, file: &str) -> Vec<Diagnostic> {
    // a generator panic is no prediction; the panic hook is left alone (files are checked in parallel, and
    // the CLI silences the default message around whole runs, as `front::quiet` does)
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::codegen::generate_rust(program).ok()));
    let Some(emitted) = rust.ok().flatten().and_then(|r| crate::ir::explain::emitted::parse_emitted(&r)) else { return Vec::new() };
    crate::ir::explain::moves::moves(&emitted).into_iter().map(|m| diagnostic(&m, source, file)).collect()
}

fn short(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

fn diagnostic(m: &Move, source: &str, file: &str) -> Diagnostic {
    let v = &m.var;
    // "used after X moved it", or for a move in a loop "X moves it in every iteration of the loop"
    let after = |by: String| if m.in_loop { format!("`{v}` is moved by {by} in every iteration of the loop") } else { format!("`{v}` is used after {by} moved it") };
    let (what, help, needle) = match &m.kind {
        MoveKind::Receiver { method } => (
            format!("{} (`{method}` takes `self` by value)", after(format!("`{v}.{}()`", short(method)))),
            format!("use the value `{}` returns, call it last, or call it on a copy: `{v}.clone().{}()`", short(method), short(method)),
            format!("{v}.{}(", short(method)),
        ),
        MoveKind::Argument { callee } => (
            format!("{} (its parameter takes ownership)", after(format!("the call to `{}`", short(callee)))),
            format!("pass a copy (`{v}.clone()`), or make the call the last use of `{v}`"),
            format!("{}(", short(callee)),
        ),
        MoveKind::Let { to } => (
            after(format!("`let {to} = {v}`")),
            format!("use `{to}` from here on, or bind a copy: `let {to} = {v}.clone()`"),
            format!("{to} = {v}"),
        ),
        MoveKind::Field { field, to } => (
            format!("{} (its field `{field}` is gone; `{v}` is used whole)", after(format!("`let {to} = {v}.{field}`"))),
            format!("copy the field (`{v}.{field}.clone()`), or stop using `{v}` as a whole after taking `{field}`"),
            format!("{v}.{field}"),
        ),
    };
    let mut d = Diagnostic::error("WJ0007", what)
        .with_help(help)
        .with_note("the interpreter would run this, but the compiled program does not build (rustc E0382)");
    if let Some(sp) = span_of(source, file, &needle) {
        d = d.at(sp.with_label("moved here"));
    }
    d
}

/// The first occurrence of `needle` starting at a word boundary.
fn span_of(source: &str, file: &str, needle: &str) -> Option<Span> {
    let byte = source
        .match_indices(needle)
        .map(|(i, _)| i)
        .find(|i| *i == 0 || !source.as_bytes()[i - 1].is_ascii_alphanumeric() && source.as_bytes()[i - 1] != b'_')?;
    let line = source[..byte].matches('\n').count() + 1;
    let line_start = source[..byte].rfind('\n').map_or(0, |n| n + 1);
    let col = source[line_start..byte].chars().count() + 1;
    let len = needle.trim_end_matches('(').chars().count();
    Some(Span { file: file.to_string(), byte_start: byte, byte_end: byte + needle.trim_end_matches('(').len(), line_start: line, col_start: col, line_end: line, col_end: col + len, label: None })
}
