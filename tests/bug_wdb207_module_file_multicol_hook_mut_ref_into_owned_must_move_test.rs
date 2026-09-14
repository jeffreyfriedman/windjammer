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

//! WDB-207: wave1 multicol hook must not pass `&mut multicol` + owned sql into
//! owned MulticolState + demoted `&str` on_simple_query.
//!
//! Product residual (pre tip-out→gen sync):
//!   `pg_wire_serve_sf1_multicol_on_simple_query(&mut multicol, sql.clone())`
//! Tip-out clean: `(multicol, sql)`. Prefer tip-out → gen sync; keep gate.

use std::path::PathBuf;

#[test]
fn wdb207_product_multicol_hook_must_not_pass_mut_ref_and_owned_sql() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("relational_pg_serve_wave1_multicol_hook_port.rs"),
        gen.join("relational/relational_pg_serve_wave1_multicol_hook_port.rs"),
        gen.join("relational_module_file/relational_pg_serve_wave1_multicol_hook_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("hook");
        let bad = text.contains("pg_wire_serve_sf1_multicol_on_simple_query(&mut multicol")
            || (text.contains("multicol_on_simple_query(&mut")
                && text.contains("sql.clone()"));
        eprintln!("WDB-207 bad={} path={}", bad, path.display());
        if path.to_string_lossy().contains("/gen/") {
            assert!(
                !bad,
                "WDB-207 RED: product hook passes &mut multicol / owned sql into owned+&str formals. {}",
                path.display()
            );
        }
    }
    if !saw {
        eprintln!("WDB-207: skip — hook missing");
    }
}
