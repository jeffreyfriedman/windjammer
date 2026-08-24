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

//! WDB-100 PRE dogfood gate: owned String reused after by-value helper must clone
//! or borrow (Phase 128 `relational_sql_pred_port` isolate-transpile E0382).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const WDB100_SOURCE: &str = r#"
use std::strings

pub fn find_word(hay: string, needle: string) -> int {
    if hay.len() == 0 { return -1 }
    0
}

pub fn after_sum(sql: string) -> string {
    let i = find_word(sql, "sum(")
    strings::substring(sql, i as usize, sql.len())
}
"#;

fn assert_wdb100_shape(rs: &str) {
    let has_clone = rs.contains("sql.clone()");
    let borrows = rs.contains("find_word(&sql") || rs.contains("substring(&sql");
    assert!(
        has_clone || borrows,
        "WDB-100: owned sql reused after find_word must clone or borrow. Got:\n{rs}"
    );
}

#[test]
fn wdb100_pre_ir_dogfood_owned_string_reuse_after_helper() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pre = manifest
        .join("..")
        .join(".worktrees")
        .join("wj-pre-ir")
        .join("target")
        .join("release")
        .join("wj");
    if !pre.exists() {
        eprintln!("skip WDB-100 PRE gate: {}", pre.display());
        return;
    }

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let src = tmp.path().join("pred.wj");
    let out = tmp.path().join("out");
    fs::write(&src, WDB100_SOURCE).unwrap();

    let build = Command::new(&pre)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
            "--library",
            "--no-generate-cargo-toml",
        ])
        .output()
        .expect("run PRE wj");
    assert!(
        build.status.success(),
        "PRE wj build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("pred.rs")).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .find_map(|e| fs::read_to_string(e.path()).ok())
            .unwrap_or_default()
    });
    assert_wdb100_shape(&rs);
}
