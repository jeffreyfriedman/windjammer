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

//! P3.740: a library `const string` (Rust `&'static str`) passed into an owned
//! `string` formal from another module must `.to_string()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn cross_module_string_const_into_owned_formal_must_own() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "uuid.wj",
        r#"
pub const NIL: string = "00000000-0000-0000-0000-000000000000"

pub fn hyphen_count(uuid: string) -> string {
    uuid
}
"#,
    );
    test.add_file(
        "caller.wj",
        r#"
use crate::uuid::NIL
use crate::uuid::hyphen_count

pub fn go() -> string {
    hyphen_count(NIL)
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.740: const string into owned formal must transpile");
    let body = map.get("caller.rs").unwrap_or_else(|| {
        panic!(
            "caller.rs missing; keys={:?}",
            map.keys().collect::<Vec<_>>()
        )
    });
    assert!(
        body.contains("NIL.to_string()") || body.contains("NIL.to_owned()"),
        "const string into owned formal must own the &'static str; got:\n{body}"
    );
}

#[test]
fn external_metadata_string_const_into_owned_formal_must_own() {
    use std::collections::HashMap;
    use windjammer::metadata::{CrateMetadata, FunctionSignature};
    use windjammer::{build_project_ext, CompilationTarget};

    let tmp = tempfile::tempdir().expect("tempdir");
    let dep = tmp.path().join("uuid_meta");
    std::fs::create_dir_all(&dep).unwrap();
    let mut functions = HashMap::new();
    functions.insert(
        "hyphen_count".to_string(),
        FunctionSignature {
            params: vec!["String".to_string()],
            formal_params: vec!["String".to_string()],
            return_type: Some("String".to_string()),
            is_associated: false,
            parent_type: None,
            param_ownership: vec!["Owned".to_string()],
            emitted_rust_ref_params: Some(vec![false]),
            string_ref_string_formal_params: None,
            forwarding_borrow_params: None,
            has_self_receiver: false,
            is_extern: false,
            is_trait_method: false,
        },
    );
    let meta = CrateMetadata {
        structs: HashMap::new(),
        functions,
        copy_structs: Vec::new(),
        string_consts: vec!["NIL".to_string()],
        version: "0.50.0".to_string(),
    };
    std::fs::write(
        dep.join("metadata.json"),
        serde_json::to_string(&meta).unwrap(),
    )
    .unwrap();

    let caller = tmp.path().join("caller.wj");
    std::fs::write(
        &caller,
        r#"
use uuid_pkg::NIL
use uuid_pkg::hyphen_count

pub fn go() -> string {
    hyphen_count(NIL)
}
"#,
    )
    .unwrap();
    let out = tmp.path().join("out");
    build_project_ext(
        &caller,
        &out,
        CompilationTarget::Rust,
        false,
        false,
        &[("uuid_pkg", dep.as_path())],
    )
    .expect("P3.740: external const string must transpile");
    let body = std::fs::read_to_string(out.join("caller.rs")).expect("caller.rs");
    assert!(
        body.contains("NIL.to_string()") || body.contains("NIL.to_owned()"),
        "metadata const string into owned formal must own; got:\n{body}"
    );
}
