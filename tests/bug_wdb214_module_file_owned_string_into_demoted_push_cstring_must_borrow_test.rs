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

//! WDB-214: owned `String` / `String::from` into demoted `&str` `push_cstring` must borrow.
//!
//! Product residual (~12× in pg_wire; dominant census `&str`←String bucket):
//!   `push_cstring(s: &str, …)` called with `f.name.clone()`, `String::from("user")`, etc.
//! Twin of WDB-203/212. Signature-driven.

use std::path::PathBuf;

#[test]
fn wdb214_tip_out_pg_wire_must_borrow_owned_string_into_demoted_push_cstring() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("relational_pg_wire_port.rs"),
        gen.join("relational/relational_pg_wire_port.rs"),
        gen.join("relational_module_file/relational_pg_wire_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pg_wire");
        let demoted = text.contains("fn push_cstring(s: &str")
            || text.contains("push_cstring(s: &str,");
        let bad_clone = text.contains("push_cstring(f.name.clone()")
            && !text.contains("push_cstring(&f.name");
        let bad_from = text.contains("push_cstring(String::from(")
            && !text.contains("push_cstring(\"user\"")
            && !text.contains("push_cstring(\"database\"");
        let bad = demoted && (bad_clone || bad_from);
        eprintln!(
            "WDB-214 demoted={} bad_clone={} bad_from={} bad={} path={}",
            demoted,
            bad_clone,
            bad_from,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-214 RED: tip-out/product passes owned String into demoted &str push_cstring. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-214: pg_wire missing");
}
