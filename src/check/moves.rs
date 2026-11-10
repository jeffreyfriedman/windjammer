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
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::codegen::generate_rust_source(program).ok()));
    let Some(emitted) = rust.ok().flatten().and_then(|r| crate::ir::explain::emitted::parse_emitted(&r)) else { return Vec::new() };
    crate::ir::explain::moves::moves(&emitted).into_iter().map(|m| diagnostic(&m, source, file)).collect()
}

fn short(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

fn diagnostic(m: &Move, source: &str, file: &str) -> Diagnostic {
    let v = &m.var;
    // the emitted code may prefix an unused binding with `_`; name it as the source does
    let src_name = |n: &str| if source.contains(n) { n.to_string() } else { n.trim_start_matches('_').to_string() };
    // "used after X moved it", or for a move in a loop "X moves it in every iteration of the loop"
    let after = |by: String| if m.in_loop { format!("`{v}` is moved by {by} in every iteration of the loop") } else { format!("`{v}` is used after {by} consumed it") };
    let (what, help, span) = match &m.kind {
        MoveKind::Receiver { method } => (
            format!("{}: `{method}` takes ownership of its receiver", after(format!("`{v}.{}()`", short(method)))),
            format!("use the value `{}` returns, or move the later use of `{v}` before the call", short(method)),
            span_of(source, file, &format!("{v}.{}(", short(method)), |_| true),
        ),
        MoveKind::Argument { callee } => (
            format!("{} (its parameter takes ownership)", after(format!("the call to `{}`", short(callee)))),
            format!("make the call the last use of `{v}`, or move the later use of `{v}` before the call"),
            // a call (not the definition) whose argument list names the variable
            span_of(source, file, &format!("{}(", short(callee)), |rest| {
                rest.split(')').next().is_some_and(|args| args.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == v))
            }),
        ),
        MoveKind::Let { to } => {
            let to = src_name(to);
            (
                after(format!("`let {to} = {v}`")),
                format!("use `{to}` from here on"),
                span_of(source, file, &format!("{to} = {v}"), |_| true),
            )
        }
        MoveKind::Field { field, to } => {
            let to = src_name(to);
            (
                format!("{} (its field `{field}` is gone; `{v}` is used whole)", after(format!("`let {to} = {v}.{field}`"))),
                format!("stop using `{v}` as a whole after taking `{field}`, or take `{field}` after the last use of `{v}`"),
                span_of(source, file, &format!("{v}.{field}"), |_| true),
            )
        }
    };
    let mut d = Diagnostic::error("WJ0007", what)
        .with_help(help)
        .with_note("the compiler copies a value that is used again, but this value holds something that cannot be copied (a channel end, a handle), so it cannot be used after it was consumed; the interpreter would run this, the compiled program would not build");
    if let Some(sp) = span {
        d = d.at(sp.with_label("moved here"));
    }
    d
}

/// The first occurrence of `needle` at a word boundary, not in a `fn` definition, for which `ok` holds of the
/// text after it.
fn span_of(source: &str, file: &str, needle: &str, ok: impl Fn(&str) -> bool) -> Option<Span> {
    let bytes = source.as_bytes();
    let byte = source.match_indices(needle).map(|(i, _)| i).find(|i| {
        let boundary = *i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_';
        boundary && !source[..*i].ends_with("fn ") && ok(&source[i + needle.len()..])
    })?;
    let line = source[..byte].matches('\n').count() + 1;
    let line_start = source[..byte].rfind('\n').map_or(0, |n| n + 1);
    let col = source[line_start..byte].chars().count() + 1;
    let text = needle.trim_end_matches('(');
    Some(Span {
        file: file.to_string(),
        byte_start: byte,
        byte_end: byte + text.len(),
        line_start: line,
        col_start: col,
        line_end: line,
        col_end: col + text.chars().count(),
        label: None,
    })
}
