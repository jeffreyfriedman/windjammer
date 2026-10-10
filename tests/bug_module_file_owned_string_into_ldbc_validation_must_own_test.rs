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
//!   `validation_entry("BFS", &bfs_path, …)` with `algorithm: String, path: String`
//! → E0308. WJ passes bare lit + owned path; sequential validates need `path.clone()` (no `&`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Entry {
    pub algorithm: string,
    pub path: string,
}

pub fn validate_bfs(path: string, src: i64) -> string {
    let mut out = path
    if src >= 0 {
        out = out + "ok"
    }
    out
}

pub fn validate_wcc(path: string) -> string {
    let mut out = path
    out = out + "wcc"
    out
}

pub fn validation_entry(algorithm: string, path: string, src: i64) -> Entry {
    let _checked = if algorithm == "BFS" {
        validate_bfs(path, src)
    } else {
        validate_wcc(path)
    }
    Entry {
        algorithm: algorithm,
        path: _checked,
    }
}

pub fn run_all(bfs_path: string, src: i64) -> Entry {
    let a = validation_entry("BFS", bfs_path, src)
    let _b = validation_entry("WCC", bfs_path, src)
    a
}
"#;

#[test]
fn module_file_owned_string_must_not_receive_ref_or_bare_lit() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");

    let owned_entry = rs.contains("fn validation_entry(algorithm: String")
        || rs.contains("fn validation_entry(mut algorithm: String");
    let owned_validate = rs.contains("fn validate_bfs(path: String")
        || rs.contains("fn validate_bfs(mut path: String");
    assert!(
        owned_entry && owned_validate,
        "expected owned String formals (owned formals stay owned):\n{rs}"
    );

    let bad_call = rs.contains("validation_entry(\"BFS\", &bfs_path")
        || rs.contains("validation_entry(\"WCC\", &bfs_path")
        || (rs.contains("validation_entry(\"BFS\"")
            && !rs.contains("\"BFS\".to_string()")
            && !rs.contains("String::from(\"BFS\")"));
    let bad_body = rs.contains("validate_bfs(&path.clone()")
        || rs.contains("validate_wcc(&path.clone()")
        || rs.contains("validate_bfs(&path)")
        || rs.contains("validate_wcc(&path)");

    assert!(
        !bad_call,
        "RED: owned validation_entry received bare lit / &path (need .to_string() + owned/clone):\n{rs}"
    );
    assert!(
        !bad_body,
        "RED: owned validate_* received &path / &path.clone() (need path.clone() / move):\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

