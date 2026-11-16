//! Entry points the test suites and tools use but no `wj` command calls (declared in docs/MODULE_MAP.toml `[library_api]`).

use crate::diagnostics::Diagnostic;
use std::path::Path;

/// `compiler_identity` of another `wj` executable (tests key entries for the binary they run).
pub fn compiler_identity_of(exe: &Path) -> String {
    super::cache::identity_of(Some(exe))
}

/// WJ0007 diagnostics for the single file at `path` (tests and tools).
pub fn predicted_moves(path: &Path) -> Vec<Diagnostic> {
    let Ok(source) = std::fs::read_to_string(path) else { return Vec::new() };
    let file = path.to_string_lossy().to_string();
    let tokens = crate::lexer::Lexer::new(&source).tokenize_with_locations();
    let Ok(program) = crate::parser::Parser::new_with_source(tokens, file.clone(), source.clone()).parse() else { return Vec::new() };
    super::moves::predict(&program, &source, &file)
}

/// Drop the installed shared analysis store (tests install their own and must leave none behind).
pub fn uninstall_shared_cache() {
    *super::shared_cache::CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = None;
}
