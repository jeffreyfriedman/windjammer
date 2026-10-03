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

//! P3.597: `for (k, v) in HashMap<int, T>` then `k == id` (owned int) must
//! auto-deref the Copy map key (`&i64 == i64` is illegal in Rust).
//!
//! Product `wj-todo-cli` TodoStore complete/remove:
//!   `for (k, v) in self.items { if k == id { … } }` → E0277.
//! Distinct from map-key get/contains_key borrow gates (call formals); this is
//! for-in binding vs owned int compare.

use std::fs;
use std::process::Command;

#[test]
fn hashmap_i64_for_in_key_eq_owned_must_auto_deref() {
    let src = tempfile::TempDir::new().expect("src");
    fs::write(
        src.path().join("lib.wj"),
        r#"use std::collections::HashMap

struct Todo {
    id: int,
    title: string,
}

struct Store {
    items: HashMap<int, Todo>,
}

pub fn has_id(store: Store, id: int) -> bool {
    for (k, v) in store.items {
        if k == id {
            let _ = v.id
            return true
        }
    }
    false
}
"#,
    )
    .unwrap();

    let out = tempfile::TempDir::new().expect("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.597 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib_rs = fs::read_to_string(out.path().join("lib.rs")).unwrap_or_default();
    eprintln!("P3.597 lib.rs:\n{lib_rs}");

    let cargo = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("wj_p3597_cargo_{}", std::process::id())),
        )
        .output()
        .expect("cargo check");
    assert!(
        cargo.status.success(),
        "P3.597 RED: for-in HashMap i64 key == owned int must auto-deref (wj-todo-cli):\n{}\n{lib_rs}",
        String::from_utf8_lossy(&cargo.stderr)
    );
}
