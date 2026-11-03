#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]
//! Drift findings as compiler tests (docs/drift/pilot4-findings.md, docs/drift/findings.md): each program in
//! `tests/drift_corpus/repros/` named below runs correctly in the interpreter, so the Rust the compiler generates
//! for it must type-check. Every test here fails today (they are listed in `.config/known-test-failures.txt`);
//! fixing the compiler makes one pass, and the drift gate's ledger line for the same program then goes stale.

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn generated_rust_typechecks(program: &str) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/drift_corpus/repros").join(program);
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("out");
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(&src).output().unwrap();
    assert!(b.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&b.stderr));
    let rs = out.join(Path::new(program).with_extension("rs"));
    let r = Command::new("rustc")
        .args(["--crate-type", "bin", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "--out-dir"])
        .arg(dir.path())
        .arg(&rs)
        .output()
        .unwrap();
    assert!(r.status.success(), "generated Rust for {program} does not type-check:\n{}", String::from_utf8_lossy(&r.stderr));
}

/// WJ0029 hint fires but `.collect::<Vec<string>>()` over `split(..)` still lowers wrongly.
#[test]
fn collect_turbofish_over_split_typechecks() {
    generated_rust_typechecks("collect_turbofish.wj");
}

/// `let mut k = 0` then `k = ...parse::<int>()...`: the literal binding is i32 against an i64 value.
#[test]
fn let_mut_zero_then_parse_typechecks() {
    generated_rust_typechecks("let_mut_zero_then_parse.wj");
}

/// `counts.get(key).unwrap_or(0)` then `n + 1`: generated `*n + 1` over an owned i64.
#[test]
fn map_get_unwrap_or_then_arithmetic_typechecks() {
    generated_rust_typechecks("map_get_unwrap_or_deref.wj");
}

/// A `Box<dyn Trait>` parameter is borrowed by inference but the argument is not coerced; the trait method
/// `area` is emitted under another name.
#[test]
fn box_dyn_trait_parameter_typechecks() {
    generated_rust_typechecks("dyn_box_param.wj");
}

/// The explicit `r as usize` feeding `chars().nth(..)` is lost.
#[test]
fn explicit_usize_cast_into_nth_is_kept() {
    generated_rust_typechecks("nth_cast.wj");
}

/// A bare unit-variant name as a pattern in a match that rebuilds the enum in a `let` is emitted as a binding (E0170).
#[test]
fn bare_unit_variant_pattern_in_let_match_typechecks() {
    generated_rust_typechecks("bare_unit_pattern_in_let_match.wj");
}

/// Two Rust types with one Windjammer name must not be reported as "expected an owned `int`, found an owned `int`".
#[test]
fn mismatch_between_integer_widths_names_both_widths() {
    use windjammer::diagnostics::rustc_text;
    let t = rustc_text::translate(Some("E0308"), "mismatched types", Some("expected `i32`, found `i64`"));
    assert_ne!(t.message, "mismatched types: expected an owned `int`, found an owned `int`");
    assert!(t.message.contains("i32") && t.message.contains("i64"), "message hides the widths: {}", t.message);
}

/// The same bare unit-variant pattern where the scrutinee of the match is itself a match.
#[test]
fn bare_unit_variant_pattern_in_nested_match_typechecks() {
    generated_rust_typechecks("bare_unit_pattern_nested.wj");
}
