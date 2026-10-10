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

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn bare_some_v_into_owned_string(rs: &str) -> bool {
    // `Some(v) => v,` / `Some(v) => v` without clone/to_owned/to_string
    for line in rs.lines() {
        let t = line.trim();
        if !(t.contains("Some(v)") && t.contains("=>")) {
            continue;
        }
        if t.contains("normalize_") || t.contains("parse_") {
            continue;
        }
        if t.contains("v.clone()")
            || t.contains("v.to_owned()")
            || t.contains("v.to_string()")
            || t.contains("(*v).clone()")
            || t.contains("String::from(v)")
        {
            continue;
        }
        // bare payload into owned String binding
        if t.contains("=> v,") || t.ends_with("=> v") || t.contains("=> v ") {
            return true;
        }
    }
    false
}

#[test]
fn hashmap_get_identity_interp_into_owned_string_must_clone() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).expect("mkdir");
    fs::write(
        src.join("lib.wj"),
        r#"
use std::collections::HashMap

pub fn read_cors(map: HashMap<string, string>) -> string {
    let cors = match map.get("cors_origin") {
        Some(v) => "${v}",
        None => "*",
    }
    cors
}
"#,
    )
    .expect("write");

    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("isolate emit:\n{rs}");
    let ok = rs.contains("v.clone()")
        || rs.contains("v.to_owned()")
        || rs.contains("v.to_string()")
        || rs.contains("(*v).clone()");
    assert!(
        ok && !bare_some_v_into_owned_string(&rs),
        "RED isolate: Some(v) => \"${{v}}\" after HashMap::get must own:\n{rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--offline"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "isolate cargo-check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
