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

//! must borrow the String key (`get(&key)` / `contains_key(&key)`), same as
//! Multipass package shape (SharedMap alias + sibling `*_get` helpers) is required.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const LIB: &str = r#"
pub mod shared
pub use shared::SharedMapSI
pub use shared::shared_map_get
pub use shared::shared_map_has
"#;

const SHARED: &str = r#"
use std::collections::HashMap
use std::sync::{Arc, Mutex}

pub struct SharedMap<K, V> {
    inner: Arc<Mutex<HashMap<K, V>>>,
}

pub type SharedMapSI = SharedMap<string, int>

/// HashMap key borrow via bare `::get` suffix scramble.
pub fn shared_int_get(n: int) -> int {
    n
}

pub fn shared_map_get(m: SharedMapSI, key: string) -> (SharedMapSI, bool, int) {
    let result = match m.inner.lock() {
        Ok(g) => {
            match g.get(key) {
                Some(v) => (true, *v),
                None => (false, 0),
            }
        },
        Err(_) => (false, 0),
    }
    (m, result.0, result.1)
}

pub fn shared_map_has(m: SharedMapSI, key: string) -> bool {
    match m.inner.lock() {
        Ok(g) => g.contains_key(key),
        Err(_) => false,
    }
}
"#;

#[test]
fn mutex_guard_hashmap_string_key_must_borrow() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), LIB).unwrap();
    fs::write(src.join("shared.wj"), SHARED).unwrap();
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
        "transpile failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("shared.rs")).unwrap_or_default();
    eprintln!("shared.rs:\n{rs}");

    let get_ok = rs.contains(".get(&key)") || rs.contains(".get(& key)");
    let contains_ok = rs.contains("contains_key(&key)") || rs.contains("contains_key(& key)");
    assert!(
        get_ok && contains_ok,
        "RED: MutexGuard HashMap get/contains_key must borrow String key:\n{rs}"
    );
    assert!(
        !rs.contains("contains_key(key.clone())"),
        "RED: must not clone key into contains_key:\n{rs}"
    );
    assert!(
        !rs.contains("get(key.to_string())") && !rs.contains("contains_key(key.to_string())"),
        "RED: must not Owned-homonym `.to_string()` the map key:\n{rs}"
    );

    let check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            windjammer::cargo_cache::verify_target_dir(),
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
        "RED: MutexGuard HashMap String key borrow must cargo-check:\n{rs}\n{err}"
    );
}
