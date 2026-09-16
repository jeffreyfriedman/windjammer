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

//! P3.308: `match map.get(k) { Some(v) => v }` with `.copied()` must not emit `*v` for Copy f32.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn match_copied_option_must_not_deref_copy_binding() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("mod.wj"),
        r#"
use std::collections::HashMap

pub fn read_score(scores: HashMap<(i32, i32), f32>, x: i32, y: i32) -> f32 {
    match scores.get((x, y)) {
        Some(v) => v,
        None => 0.0,
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
    assert!(!rs.contains("Some(v) => *v"), "copied Option arm must not deref Copy f32\n{rs}");
    assert!(rs.contains(".copied()"), "HashMap get match should use copied\n{rs}");
}
