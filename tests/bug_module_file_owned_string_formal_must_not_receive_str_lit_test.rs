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

//!
//! but test call sites pass `"fixtures/…"` bare literals
//!

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod bench
pub mod cap
"#;

const BENCH: &str = r#"
pub fn run_all_from_dir(path: string) -> u32 {
    if path == "" {
        return 0
    }
    path.len() as u32
}

pub fn scale_from_path(path: string) -> u32 {
    if path.contains("sf1") {
        return 1
    }
    0
}
"#;

const CAP: &str = r#"
use crate::bench::run_all_from_dir
use crate::bench::scale_from_path

pub fn cap_run() -> u32 {
    run_all_from_dir("fixtures/graph/example")
}

pub fn cap_scale() -> u32 {
    scale_from_path("datasets/example")
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("bench.wj", BENCH);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn module_file_owned_string_formal_must_not_receive_str_lit() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let bench_rs = map.get("bench.rs").expect("bench.rs");
    let cap_rs = map.get("cap.rs").expect("cap.rs");

    let owned_run = {
        let i = bench_rs.find("fn run_all_from_dir").unwrap_or(0);
        let sl = &bench_rs[i..bench_rs.len().min(i + 100)];
        sl.contains("path: String") && !sl.contains("path: &str")
    };
    let bad = owned_run
        && (cap_rs.contains("run_all_from_dir(\"fixtures/")
            || cap_rs.contains("scale_from_path(\"datasets/"))
        && !cap_rs.contains(".to_string()")
        && !cap_rs.contains("String::from(");

    eprintln!("bench.rs:\n{bench_rs}\ncap.rs:\n{cap_rs}");
    eprintln!("owned_run={owned_run} bad={bad}");

    if bad {
        panic!(
            "RED: owned String formal must not receive bare &str lit. \
             "
        );
    }

    test.cargo_check().expect(
        "owned String call sites must cargo-check.",
    );
}
