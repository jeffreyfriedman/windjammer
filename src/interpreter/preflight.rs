//! Load-time detection of features the interpreter does not run (the `Unsupported` entries of
//! `features.rs`), so `wj run` can choose the compiled path *before* the program has any effect.
//!
//! Each check is exact for its feature: keywords come from the compiler's lexer, call targets from the
//! shared resolver. Every error names the registry id, and `tests/interpreter_features_test.rs` asserts
//! that each `Unsupported` probe is caught here (at load), while no `Supported` probe is.

use super::error::{ErrorKind, RuntimeError, Span};
use super::project::LoadedFile;
use crate::lexer::Token;
use crate::resolve::{DefKind, Reason, Res, ResolvedProgram};

/// Prefix of every preflight message; the runner keys the compiled fallback on [`is_preflight`].
pub const TAG: &str = "not interpreted yet";

pub fn is_preflight(e: &RuntimeError) -> bool {
    e.kind == ErrorKind::Unsupported && e.message.starts_with(TAG)
}

fn err(id: &str, what: String, file: &std::path::Path, line: usize, column: usize) -> RuntimeError {
    let mut e = RuntimeError::new(ErrorKind::Unsupported, format!("{}: {} [{}]", TAG, what, id));
    e.span = Some(Span { file: file.to_string_lossy().to_string(), line, column });
    e
}

/// First registry-unsupported feature used by the program, if any.
pub fn check(files: &[LoadedFile], resolved: &ResolvedProgram<'static>, rust_crates: &[String], bridged: &dyn Fn(&str) -> bool) -> Result<(), RuntimeError> {
    for f in files {
        let Ok(src) = std::fs::read_to_string(&f.file) else { continue };
        let toks = crate::lexer::Lexer::new(&src).tokenize_with_locations();
        for w in toks.windows(2) {
            let kw = match &w[0].token {
                Token::Thread => "thread",
                Token::Async => "async",
                _ => continue,
            };
            if matches!(&w[1].token, Token::LBrace) || matches!(&w[1].token, Token::Ident(m) if m == "move") {
                return Err(err(
                    "concurrency.thread_block",
                    format!("`{} {{ }}` blocks", kw),
                    &f.file,
                    w[0].line,
                    w[0].column,
                ));
            }
        }
    }
    for call in &resolved.calls {
        let loc = call.location.as_ref();
        let at = |id: &str, what: String| {
            let (file, line, col) = loc.map(|l| (l.file.clone(), l.line, l.column)).unwrap_or_default();
            err(id, what, &file, line, col)
        };
        match &call.res {
            Res::Def(d) if resolved.collected.defs.get(*d).kind == DefKind::ExternFn => {
                let name = &resolved.collected.defs.get(*d).name;
                if !bridged(name) {
                    return Err(at("ffi.extern_fn", format!("call to `extern fn {}` (no host bridge loaded)", name)));
                }
            }
            Res::Unresolved(Reason::External(path)) => {
                let head = path.split("::").next().unwrap_or("");
                if rust_crates.iter().any(|c| c == head) {
                    return Err(at(
                        "deps.rust_crate",
                        format!("call into Rust crate dependency `{}` (`{}`)", head, path),
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}
