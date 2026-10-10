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

//! owned `String` formals must not receive `&vname`/`&lname` (Arrow FFI roundtrip).
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Batch {
    pub vertex_id_name: string,
    pub label_name: string,
}

pub fn from_arrow(vertex_id_name: string, label_name: string) -> Batch {
    Batch {
        vertex_id_name: vertex_id_name,
        label_name: label_name,
    }
}

pub fn roundtrip(batch: Batch) -> Batch {
    let vname = batch.vertex_id_name
    let lname = batch.label_name
    from_arrow(vname, lname)
}
"#;

#[test]
fn module_file_owned_string_must_not_receive_ref_names() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn from_arrow(vertex_id_name: String")
        || rs.contains("fn from_arrow(mut vertex_id_name: String");
    assert!(
        owned,
        "expected owned String formals on from_arrow:\n{rs}"
    );
    let bad = rs.contains("from_arrow(&vname") || rs.contains("from_arrow(&");
    assert!(
        !bad,
        "RED owned from_arrow received &vname/&lname:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
