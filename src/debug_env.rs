//! Developer diagnostics switches (`WJ_DEBUG_*`, `*_TRACE`, `WJ_PROFILE`).
//!
//! These only control stderr tracing and never influence generated output, so they are
//! the one sanctioned place (besides `compiler_config`) that reads the environment.
//! Behavior-changing toggles belong in `CompilerConfig`, never here.

/// True when the diagnostics variable is set to any value.
pub fn enabled(name: &str) -> bool {
    std::env::var_os(name).is_some()
}

/// True when the diagnostics variable is set to `1`, `true` or `yes`.
pub fn truthy(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}
