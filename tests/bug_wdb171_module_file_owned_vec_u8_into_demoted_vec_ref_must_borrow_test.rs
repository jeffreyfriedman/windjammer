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

//! WDB-171: owned `Vec<u8>` local / helper return into demoted `&Vec<u8>` must auto-borrow.
//!
//! Product census (~16×): `pg_wire_finish_execute(…, response: &Vec<u8>)` while call sites pass
//! owned `response` or `pg_wire_encode_select_int64_matrix(…)` temps → E0308 expected `&Vec<u8>`,
//! found `Vec<u8>` (`relational_pg_execute_port.rs`).
//!
//! WDB-126/127 tip fixtures are GREEN for `Vec<u64>` bakeoff samples; this gate is the
//! **product residual** (`Vec<u8>` wire bytes + helper-return temp). Signature-driven.
//! Prefer tip greens over dogfood/tip-cluster.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod wire
pub mod exec
"#;

const WIRE: &str = r#"
/// Read-only response consumer — tip demotes to `&Vec<u8>` (product finish_execute).
pub fn finish_execute(response: Vec<u8>) -> int {
    response.len()
}

pub fn encode_rows() -> Vec<u8> {
    vec![1, 2, 3]
}
"#;

const EXEC: &str = r#"
use crate::wire::finish_execute
use crate::wire::encode_rows

pub fn run_local() -> int {
    let response = encode_rows()
    // Product: pg_wire_finish_execute(…, response) while formal is &Vec<u8>
    finish_execute(response)
}

pub fn run_temp() -> int {
    // Product: finish_execute(…, pg_wire_encode_*(…)) owned temp into &Vec
    finish_execute(encode_rows())
}
"#;

fn wdb171_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("exec.wj", EXEC);
    test
}

#[test]
fn wdb171_module_file_owned_vec_u8_into_demoted_vec_ref_must_borrow() {
    let test = wdb171_fixture();
    let map = test
        .compile()
        .expect("WDB-171 multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let exec_rs = map.get("exec.rs").expect("exec.rs");

    eprintln!("WDB-171 wire.rs:\n{wire_rs}\nexec.rs:\n{exec_rs}");

    let demoted = wire_rs.contains("response: &Vec<u8>")
        || wire_rs.contains("response: &Vec <u8>")
        || wire_rs.contains("response:&Vec<u8>");
    let bad_local = exec_rs.contains("finish_execute(response)")
        && !exec_rs.contains("finish_execute(&response)");
    let bad_temp = exec_rs.contains("finish_execute(encode_rows())")
        && !exec_rs.contains("finish_execute(&encode_rows()");
    let good = exec_rs.contains("finish_execute(&response)")
        || exec_rs.contains("finish_execute(&encode_rows()");

    if demoted && (bad_local || bad_temp) && !good {
        panic!(
            "WDB-171 RED: demoted &Vec<u8> received owned local/temp. \
             Product: pg_wire_finish_execute(…, response) / encode_*(). Got:\n{exec_rs}\n{wire_rs}"
        );
    }

    if demoted {
        assert!(
            !bad_local && (!exec_rs.contains("finish_execute(encode_rows())") || good),
            "WDB-171: demoted &Vec<u8> must borrow owned args. Got:\n{exec_rs}"
        );
    }
}

fn wdb171_rel_gen_root() -> PathBuf {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    if tip.join("relational_pg_execute_port.rs").exists() {
        return tip;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/relational")
}

#[test]
fn wdb171_product_pg_execute_must_borrow_response_into_finish_execute() {
    let root = wdb171_rel_gen_root();
    let wire = root.join("relational_pg_wire_port.rs");
    let exec = root.join("relational_pg_execute_port.rs");
    if !wire.exists() || !exec.exists() {
        eprintln!("WDB-171: skip product gate — wire/exec missing");
        return;
    }
    let wire_text = std::fs::read_to_string(&wire).expect("wire");
    let exec_text = std::fs::read_to_string(&exec).expect("exec");
    let demoted = wire_text.contains("response: &Vec<u8>");
    let mut bad = Vec::new();
    for (i, line) in exec_text.lines().enumerate() {
        if !line.contains("pg_wire_finish_execute(") {
            continue;
        }
        // Owned trailing response arg: `, response)` or `, pg_wire_encode_…)` without `&`.
        if line.contains(", response)") && !line.contains(", &response)") {
            bad.push(format!("{}: {}", i + 1, line.trim()));
        }
        if line.contains("pg_wire_encode_")
            && line.contains("pg_wire_finish_execute")
            && !line.contains(", &pg_wire_encode_")
            && !line.contains(", &(pg_wire_encode_")
        {
            // encode call as last arg without leading &
            if let Some(idx) = line.rfind("pg_wire_encode_") {
                let before = &line[..idx];
                if before.ends_with(", ") || before.ends_with(',') {
                    bad.push(format!("{}: {}", i + 1, line.trim()));
                }
            }
        }
    }
    eprintln!(
        "WDB-171 product demoted_response={} bad={}",
        demoted,
        bad.len()
    );
    assert!(
        !(demoted && !bad.is_empty()),
        "WDB-171 RED: product pg_execute still passes owned response/encode into &Vec formal:\n{}",
        bad.join("\n")
    );
}
