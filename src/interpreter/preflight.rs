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

pub(crate) fn err(id: &str, what: String, file: &std::path::Path, line: usize, column: usize) -> RuntimeError {
    let mut e = RuntimeError::new(ErrorKind::Unsupported, format!("{}: {} [{}]", TAG, what, id));
    e.span = Some(Span { file: file.to_string_lossy().to_string(), line, column });
    e
}

/// First registry-unsupported feature used by the program, if any.
/// `std_fn`: does the interpreter implement this `std::module::function` path.
pub fn check(
    _files: &[LoadedFile],
    resolved: &ResolvedProgram<'static>,
    rust_crates: &[String],
    bridged: &dyn Fn(&str) -> bool,
    std_fn: &dyn Fn(&str) -> bool,
) -> Result<(), RuntimeError> {
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
                    return Err(at("ffi.extern_fn", format!("call to `extern fn {}` (no host bridge provides it and the C library cannot serve its signature)", name)));
                }
            }
            Res::Unresolved(Reason::External(path)) => {
                // `use std::http::*` makes every bare name look external; intrinsics (`println`...) are not std members
                if !call.callee.contains("::") && super::intrinsics::Intrinsic::from_call_name(&call.callee).is_some() {
                    continue;
                }
                let head = path.split("::").next().unwrap_or("");
                if rust_crates.iter().any(|c| c == head) {
                    return Err(at(
                        "deps.rust_crate",
                        format!("call into Rust crate dependency `{}` (`{}`)", head, path),
                    ));
                }
                // `std::module::function(..)` (module segment lowercase; type paths like `HashMap::new` are not
                // module functions) that the interpreter's std registry does not provide
                // The reason carries the import target (`std::http`); the rest of the written callee
                // (`http::serve_fn`) names the member.
                let full = match call.callee.split_once("::") {
                    Some((_, rest)) => format!("{}::{}", path, rest),
                    None => path.clone(),
                };
                let segs: Vec<&str> = full.split("::").collect();
                let lower = |s: &str| s.chars().next().is_some_and(|c| c.is_ascii_lowercase());
                let module_fn = segs.len() >= 3 && head == "std" && lower(segs[segs.len() - 2]) && lower(segs[segs.len() - 1]);
                if module_fn && !std_fn(&full) {
                    let id = if segs[1] == "http" { "std.http_network" } else { "std.module_fn" };
                    return Err(at(id, format!("`{}` is not implemented by the interpreter", full)));
                }
            }
            _ => {}
        }
    }
    Ok(())
}
