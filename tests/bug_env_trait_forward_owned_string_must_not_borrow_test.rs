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

//! FAILING REPRO — hexagonal env selector must not borrow into owned trait formals.
//!
//! Product seed adapters use `let _ = tenant_id` (discard). That must not demote the
//! trait formal to shared-ref, or env `tenant_id + ""` emits `repo.create(&_temp0, …)`
//! into a `String` slot.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/library_multipass/env_trait_forward_hex")
}

fn bad_env_forward_borrow(rs: &str) -> bool {
    rs.contains("create(&tenant_id")
        || rs.contains("get(&tenant_id")
        || rs.contains("list(&tenant_id")
        || rs.contains(".create(&_")
        || rs.contains(".get(&_")
        || rs.contains(".list(&_")
}

#[test]
fn env_trait_forward_owned_string_must_not_borrow() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    let fix = fixture_dir();
    for name in ["mod.wj", "ports.wj", "seed.wj", "postgres.wj", "env_repo.wj"] {
        fs::copy(fix.join(name), src.join(name)).unwrap();
    }
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
        .expect("wj build");
    assert!(
        build.status.success(),
        "wj --module-file build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let mut rs = String::new();
    fn collect(dir: &std::path::Path, out: &mut String) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect(&path, out);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    out.push_str(&fs::read_to_string(&path).unwrap_or_default());
                    out.push('\n');
                }
            }
        }
    }
    collect(&out, &mut rs);

    let bad = bad_env_forward_borrow(&rs);
    if bad {
        eprintln!("RED P3.267 hexagonal env trait forward must not borrow:\n{rs}");
    }
    assert!(
        !bad,
        "RED P3.267: hexagonal env forwarder must move/clone owned string, not &. Generated:\n{rs}"
    );
}
