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

//! WDB-146: multipass `--module-file` cluster emit must not flatten same-package
//! imports to `use crate::lsqb_*` (missing package path) → E0432.
//!
//! Product CQ-C5 tip cluster sync of `lsqb_benchmark_port` emitted:
//!   `use crate::lsqb_expected_count_for_scale;`
//!   `use crate::lsqb_scale_from_path;`
//! instead of `use crate::graph::lsqb_reference_counts::…`.
//!
//! Sync script can rewrite some imports, but tip should emit package-correct paths
//! when building a multi-module library cluster.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod counts
pub mod bench
"#;

const COUNTS: &str = r#"
pub fn scale_from_path(path: string) -> string {
    path
}

pub fn expected_count_for_scale(scale: string, query_id: u32) -> u64 {
    if scale == "" {
        return 0
    }
    query_id as u64
}
"#;

const BENCH: &str = r#"
use crate::counts::scale_from_path
use crate::counts::expected_count_for_scale

pub fn run(path: string) -> u64 {
    let scale = scale_from_path(path)
    expected_count_for_scale(scale, 1)
}
"#;

fn wdb146_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("counts.wj", COUNTS);
    test.add_file("bench.wj", BENCH);
    test
}

#[test]
fn wdb146_module_file_same_package_import_must_not_flatten_to_crate_root() {
    let test = wdb146_fixture();
    let map = test
        .compile()
        .expect("WDB-146 multipass compile should succeed (codegen may still be wrong)");
    let bench_rs = map.get("bench.rs").expect("bench.rs");

    let flat = bench_rs.contains("use crate::scale_from_path")
        || bench_rs.contains("use crate::expected_count_for_scale")
        || bench_rs.contains("use super::scale_from_path") == false
            && bench_rs.contains("use crate::counts::") == false
            && bench_rs.contains("scale_from_path")
            && !bench_rs.contains("use crate::counts::scale_from_path")
            && bench_rs.lines().any(|l| l.trim() == "use crate::scale_from_path;");

    let bad_flat = bench_rs
        .lines()
        .any(|l| l.trim() == "use crate::scale_from_path;" || l.trim() == "use crate::expected_count_for_scale;");

    eprintln!("WDB-146 bench.rs:\n{bench_rs}");
    eprintln!("bad_flat={bad_flat} flat={flat}");

    if bad_flat {
        panic!(
            "WDB-146 RED: same-package imports flattened to crate root. \
             Product: lsqb_benchmark_port after tip cluster sync → E0432."
        );
    }

    test.cargo_check()
        .expect("WDB-146: same-package multipass imports must cargo-check.");
}
