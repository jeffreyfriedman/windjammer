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

//! multipass demotes `Vec<u64>` formal to `&Vec<u64>` but call sites pass a
//! bare **owned local** (from `vec![…]` binding — no explicit `.clone()`) → E0308.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod bake
pub mod session
"#;

const BAKE: &str = r#"
pub fn bakeoff_run_median(samples: Vec<u64>) -> u64 {
    if samples.len() == 0 {
        return 0
    }
    samples[0]
}

pub fn bakeoff_run(bench_samples: Vec<u64>, postgres_median_nanos: u64) -> u64 {
    let m = bakeoff_run_median(bench_samples)
    m + postgres_median_nanos
}
"#;

const SESSION: &str = r#"
use crate::bake::bakeoff_run
use crate::bake::bakeoff_run_median

/// Owned locals (not demoted formals) into demoted `&Vec` callees.
pub fn record() -> u64 {
    let bench_samples = vec![10, 20]
    let postgres_samples = vec![30, 40]
    let pg_median = bakeoff_run_median(postgres_samples)
    bakeoff_run(bench_samples, pg_median)
}

pub fn cap() -> u64 {
    record()
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("bake.wj", BAKE);
    test.add_file("session.wj", SESSION);
    test
}

#[test]
fn module_file_demoted_vec_formal_must_borrow_bare_local_call_sites() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let bake_rs = map.get("bake.rs").expect("bake.rs");
    let session_rs = map.get("session.rs").expect("session.rs");

    let demoted = bake_rs.contains("samples: &Vec<u64>")
        || bake_rs.contains("bench_samples: &Vec<u64>")
        || bake_rs.contains("samples: &Vec <u64>")
        || bake_rs.contains("bench_samples: &Vec <u64>");
    let borrows = session_rs.contains("bakeoff_run_median(&postgres_samples")
        || session_rs.contains("bakeoff_run(&bench_samples")
        || session_rs.contains("&postgres_samples")
        || session_rs.contains("&bench_samples");
    let bad_bare = (session_rs.contains("bakeoff_run_median(postgres_samples)")
        || session_rs.contains("bakeoff_run(bench_samples,"))
        && !borrows;

    eprintln!("bake.rs:\n{bake_rs}\nsession.rs:\n{session_rs}");
    eprintln!("demoted={demoted} borrows={borrows} bad_bare={bad_bare}");

    if demoted && bad_bare {
        panic!(
            "RED demoted &Vec<u64> + bare owned local call sites must borrow. \
"
        );
    }

    test.cargo_check().expect(
        "RED demoted &Vec<u64> + bare owned local must compile.",
    );
}
