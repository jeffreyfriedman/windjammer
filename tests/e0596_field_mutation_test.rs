// TDD: E0596 from ownership inference — `&self` generated where Rust needs `&mut self`.
//
// Patterns (dogfooding windjammer-game):
// 1. `for e in self.items { e.field = ... }` — loop variable mutations require `&mut self` iterator.
// 2. `self.inner.m()` — resolve `Inner::m` in the registry, not an unrelated `m` on the outer impl.

#[path = "test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn rustc_compile(rust_code: &str) -> Result<(), String> {
    let test_dir = TempDir::new().expect("temp");
    let rust_file = test_dir.path().join("test.rs");
    fs::write(&rust_file, rust_code).expect("write rs");
    let out = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "-o",
            test_dir.path().join("libtest.rlib").to_str().unwrap(),
            rust_file.to_str().unwrap(),
        ])
        .output()
        .expect("rustc");
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

#[test]
fn e0596_for_loop_mutates_element_of_self_vec() {
    let src = r#"
pub struct Entity {
    pub active: bool,
}

pub struct World {
    entities: Vec<Entity>,
}

impl World {
    pub fn new() -> World {
        World { entities: Vec::new() }
    }

    pub fn cleanup(self) {
        for entity in self.entities {
            entity.active = false
        }
    }
}
"#;
    let rs = test_utils::compile_single_result(src).expect("wj compile");
    assert!(
        rs.contains("cleanup(&mut self)"),
        "expected &mut self for mutating for-loop over self.entities; got:\n{}",
        rs
    );
    rustc_compile(&rs).expect("rustc");
}

#[test]
fn e0596_negated_field_method_call_still_requires_mut_self() {
    let src = r#"
pub struct Inner {
    pub x: i32,
}

impl Inner {
    pub fn tick(self) -> bool {
        self.x = self.x + 1
        false
    }
}

pub struct Outer {
    inner: Inner,
}

impl Outer {
    pub fn step(self) {
        if !self.inner.tick() {
            return
        }
    }
}
"#;
    let rs = test_utils::compile_single_result(src).expect("wj compile");
    assert!(
        rs.contains("step(&mut self)"),
        "negated `!self.field.m()` must still infer &mut self; got:\n{}",
        rs
    );
    rustc_compile(&rs).expect("rustc");
}

#[test]
fn e0596_nested_field_method_uses_receiver_type_in_registry() {
    let src = r#"
pub struct Inner {
    pub x: i32,
}

impl Inner {
    pub fn touch(self) {
        self.x = 1
    }
}

pub struct Outer {
    inner: Inner,
}

impl Outer {
    pub fn new() -> Outer {
        Outer { inner: Inner { x: 0 } }
    }

    pub fn run(self) {
        self.inner.touch()
    }
}
"#;
    let rs = test_utils::compile_single_result(src).expect("wj compile");
    assert!(
        rs.contains("run(&mut self)"),
        "Outer::run should be &mut self when Inner::touch mutates; got:\n{}",
        rs
    );
    rustc_compile(&rs).expect("rustc");
}
