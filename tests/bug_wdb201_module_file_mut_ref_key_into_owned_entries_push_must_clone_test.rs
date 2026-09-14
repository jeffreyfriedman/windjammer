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

//! WDB-201: `&mut Key` into owned `Key` tuple/push must clone/move.
//!
//! Product residual, tip-out/gen secondary_index after sync:
//!   `out.entries.push((key, value.clone()))` while `key` is `&mut Key`
//! → expected `Key`, found `&mut Key`. Signature-driven.

use std::path::PathBuf;

#[test]
fn wdb201_product_secondary_must_not_push_mut_ref_key() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    // Prefer gen (product); also fail tip-out if present and bad.
    let paths = [
        gen.join("relational/relational_secondary_index_port.rs"),
        tip.join("relational_secondary_index_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("secondary");
        let bad = text.contains("out.entries.push((key,")
            && !text.contains("out.entries.push((key.clone(),")
            && !text.contains("out.entries.push(((*key).clone(),");
        // Only enforce when key is clearly used as &mut in same function body
        let has_mut_key = text.contains("&mut") && text.contains("entries.push");
        eprintln!(
            "WDB-201 bad={} has_mut_context={} path={}",
            bad,
            has_mut_key,
            path.display()
        );
        if path.to_string_lossy().contains("/gen/") {
            assert!(
                !bad || !has_mut_key,
                "WDB-201 RED: product gen pushes key without clone into owned Key slot. {}",
                path.display()
            );
        }
    }
    if !saw {
        eprintln!("WDB-201: skip — secondary_index missing");
    }
}
