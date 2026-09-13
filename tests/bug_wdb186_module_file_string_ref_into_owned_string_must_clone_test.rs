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

//! WDB-186: `&String` into owned `String` formal must `.clone()` / move.
//!
//! Product residual (~4× String←&String), tip-out/gen job_store:
//!   `relational_sql_parse_i64(text: String)`
//!   call `relational_sql_parse_i64(&status_text)` → E0308.
//! Distinct from WDB-170 (`&str.clone()` → need `.to_string()`) and WDB-180
//! (bare `&str` → owned String). Here the local is already `String` and the
//! call invents a shared borrow. Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod parse
pub mod decode
"#;

const PARSE: &str = r#"
/// Owns the text (product relational_sql_parse_i64).
pub fn parse_i64(text: string) -> Option<int> {
    if text.len() == 0 {
        return None
    }
    Some(text.len() as int)
}
"#;

const DECODE: &str = r#"
use crate::parse::parse_i64

pub fn pipe_field(line: string, index: int) -> string {
    let _ = index
    line
}

pub fn decode_text(wire: string) -> Option<int> {
    let status_text = pipe_field(wire, 0)
    // Product: parse_i64(&status_text) while formal owned String — must clone/move.
    parse_i64(status_text)
}
"#;

fn wdb186_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("parse.wj", PARSE);
    test.add_file("decode.wj", DECODE);
    test
}

#[test]
fn wdb186_module_file_string_ref_into_owned_string_must_clone() {
    let test = wdb186_fixture();
    let map = test
        .compile()
        .expect("WDB-186 multipass compile should succeed");
    let parse_rs = map.get("parse.rs").expect("parse.rs");
    let decode_rs = map.get("decode.rs").expect("decode.rs");

    eprintln!("WDB-186 parse.rs:\n{parse_rs}\ndecode.rs:\n{decode_rs}");

    let callee_owned = {
        let i = parse_rs.find("fn parse_i64").unwrap_or(0);
        let sl = &parse_rs[i..parse_rs.len().min(i + 100)];
        (sl.contains("text: String") || sl.contains("text:String"))
            && !(sl.contains("text: &str") || sl.contains("text:&str") || sl.contains("text: &String"))
    };
    let bad = decode_rs.contains("parse_i64(&status_text)")
        || decode_rs.contains("parse_i64(&priority_text)");
    let good = decode_rs.contains("parse_i64(status_text)")
        || decode_rs.contains("parse_i64(status_text.clone()");

    if callee_owned && bad {
        panic!(
            "WDB-186 RED: owned String formal received &String. \
             Product: relational_sql_parse_i64(&status_text). Got:\n{decode_rs}\n{parse_rs}"
        );
    }

    if callee_owned {
        assert!(
            good && !bad,
            "WDB-186: owned String formal must not get &String. Got:\n{decode_rs}"
        );
    }
}

#[test]
fn wdb186_tip_out_job_store_must_not_borrow_string_into_owned_parse() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/obs_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let job = if tip.join("observability_job_store_port.rs").exists() {
        tip.join("observability_job_store_port.rs")
    } else {
        gen.join("observability/observability_job_store_port.rs")
    };
    let peg = gen.join("relational/relational_sql_peg_port.rs");
    // parse may live in pred port
    let pred = gen.join("relational/relational_sql_pred_port.rs");
    if !job.exists() {
        eprintln!("WDB-186: skip tip-out — job_store missing");
        return;
    }
    let job_text = std::fs::read_to_string(&job).expect("job");
    let parse_src = if peg.exists() {
        std::fs::read_to_string(&peg).unwrap_or_default()
    } else {
        String::new()
    } + &if pred.exists() {
        std::fs::read_to_string(&pred).unwrap_or_default()
    } else {
        String::new()
    };
    let parse_owned = parse_src.contains("fn relational_sql_parse_i64(text: String")
        || job_text.contains("relational_sql_parse_i64"); // fall back: gate call shape
    let bad = job_text.contains("relational_sql_parse_i64(&status_text)")
        || job_text.contains("relational_sql_parse_i64(&priority_text)")
        || job_text.contains("relational_sql_parse_i64(&attempt_text)")
        || job_text.contains("relational_sql_parse_i64(&heartbeat_text)");
    eprintln!(
        "WDB-186 tip-out parse_owned={} bad={} path={}",
        parse_owned,
        bad,
        job.display()
    );
    assert!(
        !bad,
        "WDB-186 RED: tip-out/product passes &String into owned parse_i64. {}",
        job.display()
    );
}
