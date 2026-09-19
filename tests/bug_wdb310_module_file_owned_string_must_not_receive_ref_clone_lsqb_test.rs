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

//! WDB-310: owned `String` formals must not receive `&owned.clone()` (LSQB CSV loader).
//!
//! Product tip-out/gen:
//!   `lsqb_load_vertex_csv_content(graph.clone(), &filename.clone(), &content.clone())`
//!   with `filename: String, content: String` → E0308.
//! Twin of WDB-306 (bakeoff); prefer `filename.clone()` / `content.clone()` without `&`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

// Format/`len`-only helpers demote to `&str` (P3.264). Force owned String formals
// via concat into a struct field — mirrors product LSQB `filename: String, content: String`.
const SRC: &str = r#"
pub struct Loaded {
    pub filename: string,
    pub content: string,
}

pub fn load_vertex_csv_content(filename: string, content: string) -> Loaded {
    Loaded {
        filename: filename,
        content: content,
    }
}

pub fn load_csv_file(filename: string, content: string) -> Loaded {
    let a = load_vertex_csv_content(filename, content)
    let _b = load_vertex_csv_content(filename, content)
    a
}
"#;

#[test]
fn wdb310_module_file_owned_string_must_not_receive_ref_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-310 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-310 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn load_vertex_csv_content(filename: String")
        || rs.contains("fn load_vertex_csv_content(mut filename: String");
    assert!(
        owned,
        "WDB-310: expected owned String formals on load_vertex_csv_content:\n{rs}"
    );
    let bad = rs.contains("&filename.clone()")
        || rs.contains("&content.clone()")
        || rs.contains("load_vertex_csv_content(&");
    assert!(
        !bad,
        "WDB-310 RED: owned load_vertex_csv_content received &owned.clone() / &args:\n{rs}"
    );
    test.cargo_check().expect("WDB-310 cargo-check");
}

#[test]
fn wdb310_tip_out_lsqb_must_not_pass_ref_clone_into_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("lsqb_csv_loader.rs"),
        tip.join("graph/lsqb_csv_loader.rs"),
        gen.join("graph/lsqb_csv_loader.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("lsqb");
        if text.contains("&filename.clone()") || text.contains("&content.clone()") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-310: tip-out/gen lsqb_csv_loader missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-310 RED: tip-out/product passes &filename.clone()/&content.clone() into owned String in:\n  {}",
        bad_paths.join("\n  ")
    );
}
