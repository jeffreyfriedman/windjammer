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
//!   `loader.load(_temp0, &_temp1, …)` with `path: String` → E0308.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// `.len()` demotes to `&str`. Force owned String formals via struct store.
const SRC: &str = r#"
pub struct Asset {
    pub name: string,
    pub path: string,
    pub size: i64,
}

pub fn load(name: string, path: string, size: i64) -> Asset {
    Asset {
        name: name,
        path: path,
        size: size,
    }
}

pub fn load_level(level_name: string) -> Asset {
    let name = level_name + "_tilemap"
    let path = "levels/" + level_name + "/tilemap.json"
    load(name, path, 8192)
}
"#;

#[test]
fn module_file_owned_string_format_temp_must_not_receive_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn load(name: String") || rs.contains("fn load(mut name: String");
    assert!(owned, "expected owned String formals on load:\n{rs}");
    let bad = rs.contains("load(&name")
        || rs.contains("load(name, &path")
        || rs.contains("&_temp")
        || rs.contains("load(&");
    assert!(
        !bad,
        "RED: owned String load received &name/&path:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

