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

//! P3.304: `let mut i: i32` with `while (i as usize) < n` must use `i += 1`, not `i += 1 as usize`.

use std::process::Command;
use tempfile::TempDir;
use std::fs;

#[test]
fn i32_compound_add_must_not_use_usize_literal() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("mod.wj"),
        r#"
pub struct Node {
    score: f32,
}

pub fn pick_best(nodes: Vec<Node>) -> i32 {
    let mut best_idx: i32 = 0
    let mut best_f = nodes[0].score
    let mut i: i32 = 1
    while (i as usize) < nodes.len() {
        if nodes[i as usize].score < best_f {
            best_f = nodes[i as usize].score
            best_idx = i
        }
        i = i + 1
    }
    best_idx
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
        !rs.contains("+= 1 as usize") && !rs.contains("+= 1_usize"),
        "i32 increment must not use usize literal in compound add\n{rs}"
    );
}
