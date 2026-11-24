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
//! The analysis stages a cold `wj check` / `wj run` runs, pinned (docs/design/shared-program-analysis.md step 0).
//!
//! These numbers are a ledger that only shrinks: each step of the design lowers a pin and the test names the step. A pin
//! that is too high fails too (the sharing it records is gone), so the ledger cannot drift upward silently.

#[path = "common/stages.rs"]
mod stages;

use stages::{wj, SMALL};

fn small() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("p.wj"), SMALL).unwrap();
    d
}

/// The stage ledger as `name=N` pairs, in the report's order.
fn ledger(r: &stages::Ran) -> String {
    ["parse", "analyzer_pass", "analyzer_reused", "numeric", "numeric_reused", "ir_lowering", "registry_copy", "facts_build", "rust_generation"]
        .iter()
        .map(|k| format!("{k}={}", r.n(k)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Cold `wj check` of the small program. Parse 5 = the entry plus the std modules the registries read; analyzer 4 =
/// `analysis_phase` (A1), the generator in `moves::predict` (A1), `Facts::build` pass 1 (A1) and pass 2 (A2); numeric 2 =
/// `analysis_phase` and the generator; lowering 2 = `analysis_phase` and `Facts::build`; registry copies 8 = the
/// `SignatureRegistry::new` of `Facts::build` (global, per file twice) and the clone each analyzer call makes.
#[test]
fn cold_check_of_the_small_program_runs_each_stage_this_often() {
    let d = small();
    let r = wj(d.path(), &["check", "p.wj"], &[]);
    assert!(r.out.status.success(), "{}", r.stderr());
    assert_eq!(ledger(&r), PIN_CHECK_SMALL);
}

/// Cold `wj run` of the same program: the check, then the interpreter's loader parses again (3 more parses); no
/// stage beyond the parse repeats.
#[test]
fn cold_run_of_the_small_program_runs_each_stage_this_often() {
    let d = small();
    let r = wj(d.path(), &["run", "--no-lint", "p.wj"], &[]);
    assert!(r.out.status.success(), "{}", r.stderr());
    assert_eq!(r.stdout().trim(), "3");
    assert_eq!(ledger(&r), PIN_RUN_SMALL);
}

/// The generated 517-line program (`tests/fixtures/program_analysis/gen500.sh`): ten groups of structs, `Vec`,
/// `HashMap`, closures and small functions. Its `HashMap` import makes the registries read more std modules.
#[test]
fn cold_check_of_the_517_line_program_runs_each_stage_this_often() {
    let d = tempfile::tempdir().unwrap();
    std::fs::copy("tests/fixtures/program_analysis/gen500.wj", d.path().join("p.wj")).unwrap();
    let r = wj(d.path(), &["check", "p.wj"], &[]);
    assert!(r.out.status.success(), "{}", r.stderr());
    assert_eq!(ledger(&r), PIN_CHECK_517);
}

// The ledger only shrinks: step 0 of the design pins what the base does today.
const PIN_CHECK_SMALL: &str = "parse=5 analyzer_pass=4 analyzer_reused=0 numeric=2 numeric_reused=0 ir_lowering=2 registry_copy=8 facts_build=1 rust_generation=1";
const PIN_RUN_SMALL: &str = "parse=8 analyzer_pass=4 analyzer_reused=0 numeric=2 numeric_reused=0 ir_lowering=2 registry_copy=8 facts_build=1 rust_generation=1";
const PIN_CHECK_517: &str = "parse=12 analyzer_pass=6 analyzer_reused=0 numeric=3 numeric_reused=0 ir_lowering=4 registry_copy=12 facts_build=1 rust_generation=1";
