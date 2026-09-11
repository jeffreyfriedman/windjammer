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

//! WDB-127: multipass demotes `Vec<u64>` formal to `&Vec<u64>` but call sites pass a
//! bare **owned local** (from `vec![…]` binding — no explicit `.clone()`) → E0308.
//!
//! WindjammerDB CQ-C5 wave1_opt (~40× after tip cluster sync):
//!   `wave1_opt_bakeoff_run(wdb_samples, …)` while formal is `&Vec<u64>`
//!   `wave1_opt_bakeoff_run_median(postgres_samples)` while formal is `&Vec<u64>`
//!
//! Distinct from WDB-124 (`.clone()` path) and WDB-126 (direct `vec![…]` arg lit).
//! Expected: `&wdb_samples` / `&postgres_samples` (or keep owned formal).

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

pub fn bakeoff_run(wdb_samples: Vec<u64>, postgres_median_nanos: u64) -> u64 {
    let m = bakeoff_run_median(wdb_samples)
    m + postgres_median_nanos
}
"#;

const SESSION: &str = r#"
use crate::bake::bakeoff_run
use crate::bake::bakeoff_run_median

/// Owned locals (not demoted formals) into demoted `&Vec` callees.
pub fn record() -> u64 {
    let wdb_samples = vec![10, 20]
    let postgres_samples = vec![30, 40]
    let pg_median = bakeoff_run_median(postgres_samples)
    bakeoff_run(wdb_samples, pg_median)
}

pub fn cap() -> u64 {
    record()
}
"#;

fn wdb127_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("bake.wj", BAKE);
    test.add_file("session.wj", SESSION);
    test
}

#[test]
fn wdb127_module_file_demoted_vec_formal_must_borrow_bare_local_call_sites() {
    let test = wdb127_fixture();
    let map = test
        .compile()
        .expect("WDB-127 multipass compile should succeed (codegen may still be wrong)");
    let bake_rs = map.get("bake.rs").expect("bake.rs");
    let session_rs = map.get("session.rs").expect("session.rs");

    let demoted = bake_rs.contains("samples: &Vec<u64>")
        || bake_rs.contains("wdb_samples: &Vec<u64>")
        || bake_rs.contains("samples: &Vec <u64>")
        || bake_rs.contains("wdb_samples: &Vec <u64>");
    let borrows = session_rs.contains("bakeoff_run_median(&postgres_samples")
        || session_rs.contains("bakeoff_run(&wdb_samples")
        || session_rs.contains("&postgres_samples")
        || session_rs.contains("&wdb_samples");
    let bad_bare = (session_rs.contains("bakeoff_run_median(postgres_samples)")
        || session_rs.contains("bakeoff_run(wdb_samples,"))
        && !borrows;

    eprintln!("WDB-127 bake.rs:\n{bake_rs}\nsession.rs:\n{session_rs}");
    eprintln!("demoted={demoted} borrows={borrows} bad_bare={bad_bare}");

    if demoted && bad_bare {
        panic!(
            "WDB-127 RED: demoted &Vec<u64> + bare owned local call sites must borrow. \
             Product: wave1_opt_bakeoff_run / wave1_opt_bakeoff_run_median (~40 wdb-layers E0308)."
        );
    }

    // When tip already borrows, cargo-check is the regression gate.
    test.cargo_check().expect(
        "WDB-127 RED: demoted &Vec<u64> + bare owned local must compile. Product: wave1_opt_hardware_port.",
    );
}
