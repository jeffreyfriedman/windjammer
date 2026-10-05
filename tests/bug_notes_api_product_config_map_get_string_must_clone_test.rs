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

//! P3.678: notes-api `notes_config_from_map` — `HashMap::get` → `Some(v) => "${v}"`
//! into owned `string` must clone. Tip emitted bare `Some(v) => v` (`&String` → E0308).
//! Distinct from P3.638 (enum payload) / P3.671 (identity interp into owned formal).
//! Do not reshape notes-api.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn notes_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-notes-api");
        if sibling.join("src/domain/config.wj").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-notes-api");
            if uncle.join("src/domain/config.wj").exists() {
                return Some(uncle);
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

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
fn notes_api_product_config_map_get_string_must_clone() {
    let app = notes_app().unwrap_or_else(|| {
        panic!(
            "wj-notes-api src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(&app)
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build notes-api");
    assert!(
        build.status.success(),
        "P3.678 notes-api transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let config_rs = fs::read_to_string(out.join("domain").join("config.rs")).unwrap_or_default();
    let sites: Vec<&str> = config_rs
        .lines()
        .filter(|l| l.contains("Some(v)") && l.contains("=>"))
        .collect();
    eprintln!("P3.678 Some(v) sites:\n{}", sites.join("\n"));

    assert!(
        !bare_some_v_into_owned_string(&config_rs),
        "P3.678 RED: HashMap::get String payload via \"${{v}}\" must clone into owned \
         string — not bare `Some(v) => v`:\n{}",
        sites.join("\n")
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--offline"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "P3.678 notes-api cargo-check failed:\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn hashmap_get_identity_interp_into_owned_string_must_clone() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).expect("mkdir");
    // Product shape: owned `let` from match (return-match already clones on tip).
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
        "P3.678 isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.678 isolate emit:\n{rs}");
    let ok = rs.contains("v.clone()")
        || rs.contains("v.to_owned()")
        || rs.contains("v.to_string()")
        || rs.contains("(*v).clone()");
    assert!(
        ok && !bare_some_v_into_owned_string(&rs),
        "P3.678 RED isolate: Some(v) => \"${{v}}\" after HashMap::get must own:\n{rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--offline"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "P3.678 isolate cargo-check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
