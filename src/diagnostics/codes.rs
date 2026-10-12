//! The diagnostic code registry: one entry per stable code, each with an `explain` text.
//!
//! `tests/diagnostics_codes_test.rs` fails when a code literal appears in `src/` without an
//! entry here, when an entry lacks cause / fix / example, or when a front-end example does not
//! actually produce its code. Codes are never renumbered or reused.

use super::model::{Category, Origin, Severity};

pub struct CodeInfo {
    pub code: &'static str,
    pub title: &'static str,
    pub severity: Severity,
    pub category: Category,
    pub origin: Origin,
    /// `proven` | `high_confidence` | `heuristic` | `hint`
    pub maturity: &'static str,
    /// rustc codes that map to this code (empty for pure front-end / lint codes).
    pub rust_codes: &'static [&'static str],
    pub explanation: &'static str,
    pub cause: &'static str,
    pub fix: &'static str,
    /// A complete program that triggers the code.
    pub example_bad: &'static str,
    /// The same program, corrected.
    pub example_good: &'static str,
    /// True when `wj check` itself detects the example (verified by test); false when only
    /// rustc catches it today and the mapper translates it.
    pub front_end_checked: bool,
}

pub fn all() -> impl Iterator<Item = &'static CodeInfo> {
    super::codes_errors::ERRORS
        .iter()
        .chain(super::codes_lints::LINTS.iter())
}

pub fn lookup(code: &str) -> Option<&'static CodeInfo> {
    all().find(|c| c.code == code)
}

/// Map a rustc error code (`E0308`) to its Windjammer code.
pub fn from_rust_code(rust: &str) -> Option<&'static CodeInfo> {
    all().find(|c| c.rust_codes.contains(&rust))
}

/// Full `wj explain` text for a code (or a rustc code that maps to one).
pub fn explain(code: &str) -> Option<String> {
    let info = lookup(code).or_else(|| from_rust_code(code))?;
    let mut out = format!("{}: {}\n\n{}\n", info.code, info.title, info.explanation);
    out.push_str(&format!("\nCause\n  {}\n", info.cause));
    out.push_str(&format!("\nFix\n  {}\n", info.fix));
    out.push_str("\nExample (wrong)\n");
    out.push_str(&indent(info.example_bad));
    out.push_str("\nExample (corrected)\n");
    out.push_str(&indent(info.example_good));
    let how = if info.front_end_checked {
        "found by `wj check` (no rustc needed)"
    } else {
        "found by rustc and translated back to .wj source"
    };
    out.push_str(&format!("\nDetection: {how}\n"));
    if !info.rust_codes.is_empty() {
        out.push_str(&format!("Rust equivalents: {}\n", info.rust_codes.join(", ")));
    }
    Some(out)
}

fn indent(s: &str) -> String {
    s.lines().map(|l| format!("    {l}\n")).collect()
}
