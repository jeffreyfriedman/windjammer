#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! P3.575 / ecosystem `wj-csv`: `let headers = rows[0]` then later `rows[i]`
//! must clone into the owned let — not `let headers: Vec<String> = &rows[0]`
//! (E0308). Eco shape is `match parse(…) { Ok(rows) => { let headers = rows[0]; … } }`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE_PARAM: &str = r#"
pub fn parse_with_headers(rows: Vec<Vec<string>>) -> Result<(Vec<string>, Vec<Vec<string>>), string> {
    if rows.len() == 0 {
        return Err("empty")
    }
    let headers = rows[0]
    let mut data = Vec::new()
    let mut i = 1
    while i < rows.len() {
        data.push(rows[i])
        i = i + 1
    }
    Ok((headers, data))
}
"#;

/// Same control flow as `wj-csv` `parse_with_headers` (match-arm block, not fn param).
const SOURCE_MATCH_OK_ROWS: &str = r#"
fn parse_rows(text: string) -> Result<Vec<Vec<string>>, string> {
    Ok(Vec::new())
}

pub fn parse_with_headers(text: string) -> Result<(Vec<string>, Vec<Vec<string>>), string> {
    match parse_rows(text) {
        Ok(rows) => {
            if rows.len() == 0 {
                return Err("empty csv")
            }
            let headers = rows[0]
            let mut data = Vec::new()
            let mut i = 1
            while i < rows.len() {
                data.push(rows[i])
                i = i + 1
            }
            Ok((headers, data))
        },
        Err(e) => Err(e),
    }
}
"#;

fn assert_headers_cloned(label: &str, rs: &str) {
    assert!(
        !rs.contains("let headers: Vec<String> = &rows[0]")
            && !rs.contains("let headers = &rows[0]"),
        "{label} RED: owned let of indexed Vec must not bind shared ref:\n{rs}"
    );
    assert!(
        rs.contains("rows[0].clone()")
            || rs.contains("(&rows[0]).clone()")
            || rs.contains("rows[0 as usize].clone()"),
        "{label} RED: reuse of rows after index must clone into owned headers:\n{rs}"
    );
}

fn transpile_lib(source: &str) -> (String, tempfile::TempDir) {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), source).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.575 transpile failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    (rs, tmp)
}

fn cargo_check_gen(tmp: &TempDir, rs: &str) {
    let out = tmp.path().join("gen");
    let check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("wj_p3575_cargo_{}", std::process::id())),
        )
        .output()
        .expect("cargo check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(
        check.status.success(),
        "P3.575 RED: vec index reuse into owned let must cargo-check:\n{rs}\n{err}"
    );
}

#[test]
fn vec_index_reuse_must_clone_into_owned_let() {
    let (rs, tmp) = transpile_lib(SOURCE_PARAM);
    eprintln!("P3.575 param lib.rs:\n{rs}");
    assert_headers_cloned("P3.575 param", &rs);
    cargo_check_gen(&tmp, &rs);
}

#[test]
fn vec_index_reuse_in_match_ok_arm_must_clone_into_owned_let() {
    let (rs, tmp) = transpile_lib(SOURCE_MATCH_OK_ROWS);
    eprintln!("P3.575 match-Ok(rows) lib.rs:\n{rs}");
    assert_headers_cloned("P3.575 match-Ok(rows)", &rs);
    cargo_check_gen(&tmp, &rs);
}
