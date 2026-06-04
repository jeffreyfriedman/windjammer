//! Small annotation tables for Rust-backend-specific behaviors that cannot be
//! derived from method signatures alone.
//!
//! These replace the non-derivable flags from the old `method_registry` with
//! explicit, documented, minimal lists.

/// Methods whose call is a no-op when the receiver is already borrowed
/// (e.g. `.as_str()` on a value already known to be `&str`).
/// The Rust codegen strips these calls to avoid redundant conversions.
const STRIP_REDUNDANT: &[&str] = &["as_str"];

/// Windjammer syntax sugar methods that desugar to different Rust patterns:
/// - `substring(start, end)` -> `[start..end]`
/// - `slice(start, end)` -> `[start..end]`
/// - `reversed()` -> `rev().collect()`
/// - `enumerate()` -> `iter().enumerate()`
const DESUGARED: &[&str] = &["substring", "slice", "reversed", "enumerate"];

pub fn is_strip_redundant(method: &str) -> bool {
    STRIP_REDUNDANT.contains(&method)
}

pub fn is_desugared_method(method: &str) -> bool {
    DESUGARED.contains(&method)
}
