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

//!
//!   `out.entries.push((key, value.clone()))` while `key` is `&mut Key`
//! → expected `Key`, found `&mut Key`. Signature-driven (Vec::push owns T).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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

fn fixture() -> MultiFileTest {
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
fn module_file_reused_key_into_owned_entries_push_must_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let rs = map.get("index.rs").expect("index.rs");
    eprintln!("isolate index.rs:\n{rs}");
    let mut_ref_formal = rs.contains("key: &mut Key") || rs.contains("key:&mut Key");
    let shared_ref_formal = rs.contains("key: &Key") || rs.contains("key:&Key");
    let bare_push = rs.contains("out.entries.push((key,")
        && !rs.contains("out.entries.push((key.clone()")
        && !rs.contains("out.entries.push(((*key).clone()");
    assert!(
        push_owns_key(rs),
        "RED: reused key into owned Vec<(Key, Value)> push must clone or stay owned. Got:\n{rs}"
    );
    assert!(
        !(mut_ref_formal && bare_push) && !(shared_ref_formal && bare_push),
        "RED: demoted &Key / &mut Key pushed bare into owned slot. Got:\n{rs}"
    );
    test.cargo_check()
        .expect("reused key into owned entries.push must cargo-check");
}

