//! Uses after a move that rustc would reject (E0382, reported as WJ0007), found before an interpreted run.
//!
//! The interpreter copies values, so a program that uses a value after moving it runs interpreted and
//! fails only when built. The prediction reads the Rust the compiler generates for the file (in-process,
//! no cargo; `ir::explain::moves`), so it follows the compiler's own decisions about receivers, clones and
//! `Copy` types. A file the generator cannot handle on its own (one module of a project) is skipped.

use crate::diagnostics::{Diagnostic, Span};
use std::path::Path;

/// WJ0007 diagnostics for the uses after a move in the single file at `path`.
pub fn predicted_moves(path: &Path) -> Vec<Diagnostic> {
    let Ok(source) = std::fs::read_to_string(path) else { return Vec::new() };
    let file = path.to_string_lossy().to_string();
    let tokens = crate::lexer::Lexer::new(&source).tokenize_with_locations();
    let Ok(program) = crate::parser::Parser::new_with_source(tokens, file.clone(), source.clone()).parse() else { return Vec::new() };
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::codegen::generate_rust(&program).ok()));
    std::panic::set_hook(previous);
    let Some(emitted) = rust.ok().flatten().and_then(|r| crate::ir::explain::emitted::parse_emitted(&r)) else { return Vec::new() };
    crate::ir::explain::moves::receiver_moves(&emitted)
        .into_iter()
        .map(|m| {
            let method = m.method.rsplit("::").next().unwrap_or(&m.method).to_string();
            let mut d = Diagnostic::error(
                "WJ0007",
                format!(
                    "`{}` is used after `{}.{}()` consumed it: `{}` takes ownership of its receiver",
                    m.var, m.var, method, m.method
                ),
            )
            .with_help(format!(
                "use the value `{method}` returns (`let next = {v}.{method}()`), move the later use of `{v}` before the call, or call it on a copy: `{v}.clone().{method}()`",
                v = m.var
            ))
            .with_note("the interpreter would run this, but the compiled program does not build: a value cannot be used after a call that took ownership of it");
            if let Some(sp) = call_span(&source, &file, &m.var, &method) {
                d = d.at(sp.with_label("moved here"));
            }
            d
        })
        .collect()
}

/// The first `var.method(` in the source.
fn call_span(source: &str, file: &str, var: &str, method: &str) -> Option<Span> {
    let needle = format!("{var}.{method}(");
    let byte = source.match_indices(&needle).map(|(i, _)| i).find(|i| *i == 0 || !source.as_bytes()[i - 1].is_ascii_alphanumeric() && source.as_bytes()[i - 1] != b'_')?;
    let line = source[..byte].matches('\n').count() + 1;
    let line_start = source[..byte].rfind('\n').map_or(0, |n| n + 1);
    let col = source[line_start..byte].chars().count() + 1;
    let len = needle.len() - 1;
    Some(Span { file: file.to_string(), byte_start: byte, byte_end: byte + len, line_start: line, col_start: col, line_end: line, col_end: col + len, label: None })
}
