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
    feature = "codegen_tests",
))]

//! WDB-241: demoted `&Vec<u32>` into owned `ecs_soa_archetype_new` must clone.
//!
//! Product residual tip-out/gen ecs_soa_port (~54× Vec←&Vec class):
//!   `ecs_soa_archetype_new(component_ids: Vec<u32>)` called with `&ids` → E0308.
//! Twin of WDB-224 (dremel struct Vec). Signature-driven: `ids.clone()` / `ids.to_vec()`.

use std::path::PathBuf;

#[test]
fn wdb241_tip_out_ecs_must_clone_demoted_ids_into_owned_archetype_new() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("ecs_soa_port.rs"),
        gen.join("ecs/ecs_soa_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("ecs");
        let owned = text.contains("fn ecs_soa_archetype_new(component_ids: Vec<u32>");
        let bad = text.contains("ecs_soa_archetype_new(&ids)")
            && !text.contains("ecs_soa_archetype_new(ids.clone())")
            && !text.contains("ecs_soa_archetype_new(ids.to_vec())");
        eprintln!("WDB-241 owned={} bad={} path={}", owned, bad, path.display());
        if owned {
            assert!(
                !bad,
                "WDB-241 RED: tip-out/product passes &ids into owned Vec archetype_new. {}",
                path.display()
            );
        }
    }
    assert!(saw, "WDB-241: ecs_soa_port missing");
}
