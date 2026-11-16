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

pub use super::test_api::predicted_moves;

/// WJ0007 diagnostics for one parsed file.
pub fn predict(program: &crate::parser::Program<'_>, source: &str, file: &str) -> Vec<Diagnostic> {
    let rust = generate(program);
    remember(file, source, &rust);
    rust.map_or_else(Vec::new, |rust| predict_from(&rust, source, file))
}

/// The Rust generated for a file's text, kept for the width trigger of the same run (`widths::generated_rust`
/// would generate it a second time from a fresh parse of the same text). Taken once, so nothing outlives its use;
/// at most [`REMEMBERED_FILES`] are kept.
static GENERATED: std::sync::Mutex<Vec<(String, String, Option<String>)>> = std::sync::Mutex::new(Vec::new());
const REMEMBERED_FILES: usize = 512;

fn remember(file: &str, source: &str, rust: &Option<String>) {
    let mut kept = GENERATED.lock().unwrap_or_else(|e| e.into_inner());
    if kept.len() < REMEMBERED_FILES {
        kept.push((file.to_string(), source.to_string(), rust.clone()));
    }
}

/// A file whose generated Rust nothing needs to read (`project::nothing_to_predict`): remembered as "no Rust", so the
/// width trigger of the same run does not generate it either.
pub fn skip(file: &str, source: &str) {
    remember(file, source, &None);
}

/// The Rust `predict` generated for exactly this file text, if this run did (`Some(None)`: it tried and failed).
pub fn take_generated(file: &str, source: &str) -> Option<Option<String>> {
    let mut kept = GENERATED.lock().unwrap_or_else(|e| e.into_inner());
    let at = kept.iter().position(|(f, s, _)| f == file && s == source)?;
    Some(kept.swap_remove(at).2)
}

/// The Rust the compiler generates for one parsed file, in-process; `None` when the generator fails.
pub fn generate(program: &crate::parser::Program<'_>) -> Option<String> {
    // a generator panic is no prediction; the panic hook is left alone (files are checked in parallel, and
    // the CLI silences the default message around whole runs, as `front::quiet` does)
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::codegen::generate_rust_source(program).ok()));
    rust.ok().flatten()
}

/// WJ0007 diagnostics from a file's generated Rust.
pub fn predict_from(rust: &str, source: &str, file: &str) -> Vec<Diagnostic> {
    let Some(emitted) = crate::ir::explain::emitted::parse_emitted(rust) else { return Vec::new() };
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
                // `let to = v`, or a branch of `let to = if .. { v } else { .. }`: the `v` after `let to`
                word_after(source, file, &format!("let {to}"), v).or_else(|| span_of(source, file, &format!("{to} = {v}"), |_| true)),
            )
        }
        MoveKind::Field { field, to } => {
            let to = src_name(to);
            (
                format!("{} (its field `{field}` is gone; `{v}` is used whole)", after(format!("`let {to} = {v}.{field}`"))),
                format!("stop using `{v}` as a whole after taking `{field}`, or take `{field}` after the last use of `{v}`"),
                // the `v.field` of `let to = v.field`, not a `v.field` read elsewhere (another function's body)
                word_after(source, file, &format!("let {to}"), &format!("{v}.{field}")).or_else(|| span_of(source, file, &format!("{v}.{field}"), |_| true)),
            )
        }
    };
    let mut d = Diagnostic::error("WJ0007", what)
        .with_help(help)
        .with_note("the compiler copies a value that is used again, but this value holds something that cannot be copied (a channel end, a handle), so it cannot be used after it was consumed; the interpreter would run this, the compiled program would not build");
    if let Some(sp) = span {
        if m.in_loop {
            if let Some(l) = enclosing_loop(source, file, sp.byte_start) {
                d = d.with_secondary(l.with_label("the loop that repeats the move"));
            }
        }
        d = d.at(sp.with_label("moved here"));
    }
    d
}

/// The first whole-word `word` on the line of the first `anchor` (`let arm` .. `inbox`), outside comments.
fn word_after(source: &str, file: &str, anchor: &str, word: &str) -> Option<Span> {
    let whole = |rest: &str| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_');
    let a = span_of(source, file, anchor, whole).or_else(|| span_of(source, file, &anchor.replacen("let ", "let mut ", 1), whole))?;
    let line_end = source[a.byte_end..].find('\n').map_or(source.len(), |n| a.byte_end + n);
    let rest = &source[a.byte_end..line_end];
    let bytes = rest.as_bytes();
    let is_word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let off = rest.match_indices(word).map(|(i, _)| i).find(|&i| {
        (i == 0 || !is_word(bytes[i - 1])) && bytes.get(i + word.len()).is_none_or(|c| !is_word(*c))
    })?;
    let byte = a.byte_end + off;
    let col = a.col_start + source[a.byte_start..byte].chars().count();
    Some(Span { byte_start: byte, byte_end: byte + word.len(), col_start: col, col_end: col + word.chars().count(), ..a })
}

/// The header of the innermost loop around `byte`: the nearest earlier line, less indented than the move's line, that starts
/// with `for`, `while` or `loop`.
fn enclosing_loop(source: &str, file: &str, byte: usize) -> Option<Span> {
    let indent = |l: &str| l.len() - l.trim_start().len();
    let line_start = source[..byte].rfind('\n').map_or(0, |n| n + 1);
    let mut limit = indent(&source[line_start..]);
    let mut end = line_start;
    while end > 0 {
        let start = source[..end - 1].rfind('\n').map_or(0, |n| n + 1);
        let line = &source[start..end - 1];
        let t = line.trim_start();
        if !t.is_empty() && indent(line) < limit {
            if t.starts_with("for ") || t.starts_with("while ") || t.starts_with("loop") {
                let col = indent(line) + 1;
                let kw = t.split_whitespace().next().unwrap_or("for");
                let b = start + indent(line);
                let line_no = source[..b].matches('\n').count() + 1;
                return Some(Span {
                    file: file.to_string(),
                    byte_start: b,
                    byte_end: b + kw.len(),
                    line_start: line_no,
                    col_start: col,
                    line_end: line_no,
                    col_end: col + kw.chars().count(),
                    label: None,
                });
            }
            if t.starts_with("fn ") || t.starts_with("pub fn ") {
                return None;
            }
            limit = indent(line);
        }
        end = start;
    }
    None
}

/// Byte `at` lies in a `//` comment or a string literal of its line (the call the prediction names is code).
fn in_comment_or_string(source: &str, at: usize) -> bool {
    let line_start = source[..at].rfind('\n').map_or(0, |n| n + 1);
    let (mut in_str, mut escaped) = (false, false);
    let mut prev = ' ';
    for c in source[line_start..at].chars() {
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '/' && prev == '/' {
            return true;
        }
        prev = c;
    }
    in_str
}

/// The first occurrence of `needle` at a word boundary, not in a `fn` definition, for which `ok` holds of the
/// text after it.
fn span_of(source: &str, file: &str, needle: &str, ok: impl Fn(&str) -> bool) -> Option<Span> {
    let bytes = source.as_bytes();
    let byte = source.match_indices(needle).map(|(i, _)| i).find(|i| {
        let boundary = *i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_';
        boundary && !source[..*i].ends_with("fn ") && !in_comment_or_string(source, *i) && ok(&source[i + needle.len()..])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_in_a_comment_or_a_string_is_not_the_call() {
        let src = "// the second take(h) is a use after move\nlet s = \"take(h)\"\nlet a = take(h) // take(h)\n";
        let at = |needle: &str, nth: usize| src.match_indices(needle).nth(nth).unwrap().0;
        assert!(in_comment_or_string(src, at("take(h)", 0)), "comment line");
        assert!(in_comment_or_string(src, at("take(h)", 1)), "string literal");
        assert!(!in_comment_or_string(src, at("take(h)", 2)), "code");
        assert!(in_comment_or_string(src, at("take(h)", 3)), "trailing comment");
    }

    #[test]
    fn the_prediction_points_at_the_call_not_at_a_comment_mentioning_it() {
        let src = "// take(h) twice\nfn main() {\n    let a = take(h)\n}\n";
        let sp = span_of(src, "t.wj", "take(", |rest| rest.starts_with("h)")).expect("found");
        assert_eq!((sp.line_start, sp.col_start), (3, 13));
    }
}
