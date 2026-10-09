#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! WDB-204: `u64` compare must not emit `0_usize` literal.
//!
//! Product residual (~3× u64←usize), tip-out/gen sysbench:
//!   `if median == 0_usize` while `median: u64` → E0308 + E0277.
//! Zero literal must match u64 (`0` / `0_u64`).

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use windjammer::build_project;
use windjammer::CompilationTarget;

/// Tuple field `pair.0` from `(u64, bool)` compared to `0` must suffix `u64`.
/// `samples.len() == 0` may stay `usize`.
#[test]
fn wdb204_tuple_u64_field_compare_zero_must_not_emit_usize() {
    let source = r#"
pub fn quiet(samples: Vec<u64>) -> (u64, bool) {
    if samples.len() == 0 {
        return (0, false)
    }
    (samples[0], false)
}

pub fn verdict(samples: Vec<u64>) -> bool {
    let pair = quiet(samples)
    let median = pair.0
    if median == 0 {
        return false
    }
    true
}
"#;
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("test.wj");
    fs::write(&wj, source).unwrap();
    let out = tmp.path().join("build");
    build_project(&wj, &out, CompilationTarget::Rust, false).expect("transpile");
    let rust = fs::read_to_string(out.join("test.rs")).expect("test.rs");
    eprintln!("WDB-204 isolate:\n{rust}");
    let median_cmp = rust.lines().find(|l| l.contains("median")).unwrap_or("");
    assert!(
        !median_cmp.contains("0_usize"),
        "WDB-204 RED: u64 tuple field compared to 0_usize:\n{rust}"
    );
    test_utils::cargo_check_generated(&out);
}

#[test]
fn wdb204_tip_out_sysbench_must_not_compare_u64_to_usize_zero() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let sysbench = if tip.join("sysbench_opt_port.rs").exists() {
        tip.join("sysbench_opt_port.rs")
    } else {
        gen.join("relational/sysbench_opt_port.rs")
    };
    if !sysbench.exists() {
        eprintln!("WDB-204: skip — sysbench missing");
        return;
    }
    let text = std::fs::read_to_string(&sysbench).expect("sysbench");
    // Only u64==0_usize is RED — loop indices may legitimately use `0_usize`.
    let bad = text.contains("== 0_usize") || text.contains("!= 0_usize");
    eprintln!("WDB-204 tip-out bad={} path={}", bad, sysbench.display());
    assert!(
        !bad,
        "WDB-204 RED: tip-out/product compares u64 median to 0_usize. {}",
        sysbench.display()
    );
}
