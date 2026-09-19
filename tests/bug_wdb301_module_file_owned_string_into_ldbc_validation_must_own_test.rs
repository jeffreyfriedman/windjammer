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

//! WDB-301: owned `string` formals must not receive `&str` lit / `&String` / `&path.clone()`.
//!
//! Product tip-out/gen LDBC validation (~12× + body probes):
//!   `validation_entry("BFS", &bfs_path, …)` with `algorithm: String, path: String`
//!   body `ldbc_validation_validate_bfs(&path.clone(), …)` with owned `path: String`
//! → E0308. WJ passes bare lit + owned path; sequential validates need `path.clone()` (no `&`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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
fn wdb301_module_file_owned_string_must_not_receive_ref_or_bare_lit() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-301 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-301 MultiFile lib.rs:\n{rs}");

    let owned_entry = rs.contains("fn validation_entry(algorithm: String")
        || rs.contains("fn validation_entry(mut algorithm: String");
    let owned_validate = rs.contains("fn validate_bfs(path: String")
        || rs.contains("fn validate_bfs(mut path: String");
    assert!(
        owned_entry && owned_validate,
        "WDB-301: expected owned String formals (product LDBC keeps owned):\n{rs}"
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
        "WDB-301 RED: owned validation_entry received bare lit / &path (need .to_string() + owned/clone):\n{rs}"
    );
    assert!(
        !bad_body,
        "WDB-301 RED: owned validate_* received &path / &path.clone() (need path.clone() / move):\n{rs}"
    );
    test.cargo_check().expect("WDB-301 cargo-check");
}

#[test]
fn wdb301_tip_out_ldbc_validation_must_own_string_args() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_ldbc_validation_port.rs"),
        tip.join("graph/graph_ldbc_validation_port.rs"),
        gen.join("graph/graph_ldbc_validation_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("ldbc");
        let bad = text.contains("validation_entry(\"BFS\", &bfs_path")
            || text.contains("validate_bfs(&path.clone()")
            || text.contains("ldbc_validation_validate_bfs(&path.clone()")
            || text.contains("ldbc_validation_validate_pagerank(&path.clone()");
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-301: tip-out/gen LDBC validation port missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-301 RED: tip-out/product passes &str/&String into owned LDBC string formals in:\n  {}",
        bad_paths.join("\n  ")
    );
}
