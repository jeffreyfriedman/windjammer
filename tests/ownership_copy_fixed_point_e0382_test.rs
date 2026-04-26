//! TDD: E0382 / Copy registry alignment — empty structs and declaration order.
//!
//! Root causes fixed:
//! 1. `main.rs` PASS 0 skipped empty structs; they are Copy in Rust and in codegen.
//! 2. Analyzer Phase 0 used a single forward pass, so a struct referencing a later
//!    Copy struct was never marked Copy → wrong `self` inference → E0382.

use std::process::Command;

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    let wj_file = dir.path().join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.path().to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let main_rs = if dir.path().join("src").join("main.rs").exists() {
        dir.path().join("src").join("main.rs")
    } else {
        dir.path().join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.path().join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

/// E0382: empty struct is Copy; getter returning Copy type should use `&self` so the
/// receiver can be used again.
#[test]
fn test_empty_struct_copy_self_borrow_for_getter() {
    let source = r#"
pub struct Marker {}

impl Marker {
    pub fn tag(self) -> Marker {
        Marker {}
    }
}

fn main() {
    let m = Marker {}
    let _ = m.tag()
    let _ = m.tag()
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        rs.contains("fn tag(&self)"),
        "Empty Marker is Copy; tag should take &self. Got:\n{}",
        rs
    );
    assert!(compiles, "Must compile without E0382. Rust:\n{}", rs);
}

/// E0382: `Wrapper` appears before `Inner` in source; fixed-point marks both Copy so
/// `inner_copy` can use `&self` and `w` is usable twice.
#[test]
fn test_copy_struct_forward_reference_declaration_order() {
    let source = r#"
pub struct Wrapper {
    inner: Inner
}

pub struct Inner {
    v: u32
}

impl Wrapper {
    pub fn inner_copy(self) -> Inner {
        self.inner
    }
}

fn main() {
    let w = Wrapper { inner: Inner { v: 1 } }
    let _ = w.inner_copy()
    let _ = w.inner_copy()
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        rs.contains("fn inner_copy(&self)"),
        "Wrapper should be Copy after Inner; method should be &self. Got:\n{}",
        rs
    );
    assert!(compiles, "Must compile without E0382. Rust:\n{}", rs);
}

/// E0507-style guard: registry must treat empty structs as Copy (matches codegen derive).
#[test]
fn test_empty_struct_derives_copy_in_output() {
    let source = r#"
pub struct UnitLike {}

pub fn main() {
    let _ = UnitLike {}
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        rs.contains("derive(") && rs.contains("Copy"),
        "Empty struct should auto-derive Copy like codegen. Got:\n{}",
        rs
    );
    assert!(compiles, "Must compile. Rust:\n{}", rs);
}
