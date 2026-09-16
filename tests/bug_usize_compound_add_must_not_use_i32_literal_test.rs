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

//! P3.307: usize loop counter must not use `1 as i32` on increment (astar_grid).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn usize_compound_add_must_not_use_i32_literal() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("mod.wj"),
        r#"
pub fn walk_neighbors(neighbors: Vec<i32>) {
    let mut ni = 0
    while ni < neighbors.len() {
        let _ = neighbors[ni]
        ni = ni + 1
    }
}
"#,
    )
    .unwrap();
    let out = tmp.path().join("build");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let rs = fs::read_to_string(out.join("mod.rs")).unwrap_or_default()
        + &fs::read_dir(&out).unwrap().filter_map(|e| {
            let p = e.ok()?.path();
            if p.extension()?.to_str()? == "rs" && p.file_name()? != "mod.rs" {
                Some(fs::read_to_string(p).ok()?)
            } else {
                None
            }
        }).collect::<String>();
    assert!(
        !rs.contains("+= 1 as i32") && !rs.contains("+ 1 as i32"),
        "usize counter increment must not use i32 literal cast\n{rs}"
    );
}
