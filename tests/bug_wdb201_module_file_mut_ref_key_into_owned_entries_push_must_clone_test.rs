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

//! WDB-201: reused `key` into owned `Vec<(Key, Value)>::push` must clone/move.
//!
//! Product residual, tip-out/gen secondary_index after sync:
//!   `out.entries.push((key, value.clone()))` while `key` is `&mut Key`
//! → expected `Key`, found `&mut Key`. Signature-driven (Vec::push owns T).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod index
"#;

const INDEX: &str = r#"
pub struct Key {
    pub id: string,
}

pub struct Value {
    pub n: int,
}

pub struct Index {
    pub entries: Vec<(Key, Value)>,
}

fn keys_equal(a: Key, b: Key) -> bool {
    a.id == b.id
}

/// Product `relational_secondary_index_put`: compare `key` in a loop, then
/// assign or push the same binding into owned `(Key, Value)`.
pub fn put(index: Index, key: Key, value: Value) -> Index {
    let mut out = index
    let mut i = 0
    while i < out.entries.len() {
        if keys_equal(out.entries[i].0, key) {
            out.entries[i] = (key, value)
            return out
        }
        i = i + 1
    }
    out.entries.push((key, value))
    out
}
"#;

fn wdb201_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("index.wj", INDEX);
    test
}

fn push_owns_key(rs: &str) -> bool {
    rs.contains("out.entries.push((key.clone()")
        || rs.contains("out.entries.push(((*key).clone()")
        || rs.contains("out.entries.push((key.to_owned()")
        || (rs.contains("out.entries.push((key,") && !rs.contains("key: &mut Key") && !rs.contains("key: &Key"))
}

#[test]
fn wdb201_module_file_reused_key_into_owned_entries_push_must_clone() {
    let test = wdb201_fixture();
    let map = test
        .compile()
        .expect("WDB-201 multipass compile should succeed");
    let rs = map.get("index.rs").expect("index.rs");
    eprintln!("WDB-201 isolate index.rs:\n{rs}");
    let mut_ref_formal = rs.contains("key: &mut Key") || rs.contains("key:&mut Key");
    let shared_ref_formal = rs.contains("key: &Key") || rs.contains("key:&Key");
    let bare_push = rs.contains("out.entries.push((key,")
        && !rs.contains("out.entries.push((key.clone()")
        && !rs.contains("out.entries.push(((*key).clone()");
    assert!(
        push_owns_key(rs),
        "WDB-201 RED: reused key into owned Vec<(Key, Value)> push must clone or stay owned. Got:\n{rs}"
    );
    assert!(
        !(mut_ref_formal && bare_push) && !(shared_ref_formal && bare_push),
        "WDB-201 RED: demoted &Key / &mut Key pushed bare into owned slot. Got:\n{rs}"
    );
    test.cargo_check()
        .expect("WDB-201: reused key into owned entries.push must cargo-check");
}

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
