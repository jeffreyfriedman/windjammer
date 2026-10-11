//! Registry of lint (W-code) diagnostics. `docs/reference/errors.md` is generated from it.
//!
//! Every `"W####"` string literal anywhere in `src/` (linter, parser, CLI) must be registered here; `tests/docs_truth_test.rs`
//! scans the sources to enforce it. W0007/W0008 are registered in the error catalog
//! (`intrinsics::register_catalog`) and are not repeated here.

#[derive(Debug, Clone, Copy)]
pub struct LintCode {
    pub code: &'static str,
    pub name: &'static str,
    /// "error" | "warning" | "note"
    pub default_level: &'static str,
    pub summary: &'static str,
}

pub const LINT_CODES: &[LintCode] = &[
    LintCode { code: "W0001", name: "explicit-ownership", default_level: "error", summary: "explicit ownership annotation (`&`, `&mut`) on a parameter" },
    LintCode { code: "W0002", name: "explicit-unwrap", default_level: "error", summary: "explicit `.unwrap()` / `.expect()` call" },
    LintCode { code: "W0003", name: "explicit-iter", default_level: "error", summary: "explicit `.iter()` / `.iter_mut()` call" },
    LintCode { code: "W0004", name: "explicit-borrow", default_level: "error", summary: "explicit borrow in a function call" },
    LintCode { code: "W0005", name: "explicit-clone", default_level: "warning", summary: "explicit `.clone()` call" },
    LintCode { code: "W0006", name: "string-literal-to-string", default_level: "error", summary: "string literal `.to_string()` is Rust leakage" },
    LintCode { code: "W0010", name: "non-canonical-string-type", default_level: "error", summary: "`String`/`&str` spelled in source; Windjammer has one string type, `string` (emitted by the parser)" },
];
