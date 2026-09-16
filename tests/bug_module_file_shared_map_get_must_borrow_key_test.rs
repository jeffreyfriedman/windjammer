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

//! FAILING REPRO — library multipass SharedMap still emits `key.to_string()` on get/has.
//!
//! Isolate `compile_single` for the same fixture is tip GREEN (P3.288), but
//! `wj build --library --module-file` for `wj-sync` SharedMap emits
//! `get(key.to_string())` / `contains_key(key.to_string())` → E0308.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SHARED_MAP_MODULE: &str = r#"
use std::collections::HashMap
use std::sync::{Arc, Mutex}

struct MapCell {
    data: HashMap<string, int>,
}

pub struct SharedMap {
    inner: Arc<Mutex<MapCell>>,
}

pub fn shared_map_new() -> SharedMap {
    SharedMap {
        inner: Arc::new(Mutex::new(MapCell {
            data: HashMap::new(),
        })),
    }
}

pub fn shared_map_insert(m: SharedMap, key: string, value: int) -> SharedMap {
    match m.inner.lock() {
        Ok(mut g) => {
            g.data.insert(key, value)
        }
        Err(_) => {}
    }
    m
}

pub fn shared_map_get(m: SharedMap, key: string) -> (SharedMap, bool, int) {
    let result = match m.inner.lock() {
        Ok(g) => {
            match g.data.get(key) {
                Some(v) => (true, v),
                None => (false, 0),
            }
        }
        Err(_) => (false, 0),
    }
    (m, result.0, result.1)
}

pub fn shared_map_has(m: SharedMap, key: string) -> bool {
    let out = match m.inner.lock() {
        Ok(g) => g.data.contains_key(key),
        Err(_) => false,
    }
    out
}
"#;

#[test]
fn module_file_shared_map_get_must_borrow_key() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), SHARED_MAP_MODULE).unwrap();
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
        "library build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    assert!(
        !generated.contains("get(key.to_string())")
            && !generated.contains("contains_key(key.to_string())"),
        "RED P3.301: multipass SharedMap get/has must borrow key:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (SharedMap multipass):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
