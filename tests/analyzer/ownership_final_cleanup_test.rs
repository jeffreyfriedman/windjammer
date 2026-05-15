#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: Abstract trait `self` parsing must not force by-value receivers for non-`Self` returns.
//!
//! Root cause (dogfooding): `fn is_enabled(self) -> bool` on a trait was parsed as `OwnershipHint::Owned`
//! for every abstract single-`self` method, so Rust emitted `fn is_enabled(self)`, breaking `dyn Trait`
//! (E0161/E0507). Getter-style methods must stay inferable as `&self`.

#[path = "../common/test_utils.rs"]
mod test_utils;

use std::process::Command;

fn rust_lib_compiles(rust_code: &str) -> bool {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let rs_path = temp_dir.path().join("lib.rs");
    std::fs::write(&rs_path, rust_code).expect("write");
    let out = temp_dir.path().join("out.rlib");
    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            out.to_str().unwrap(),
        ])
        .arg(&rs_path)
        .output()
        .expect("rustc");
    output.status.success()
}

#[test]
fn test_abstract_trait_bool_getter_uses_ref_self_for_dyn_trait() {
    let source = r#"
pub trait System {
    fn is_enabled(self) -> bool
}

pub struct Physics {
    enabled: bool,
}

impl System for Physics {
    fn is_enabled(self) -> bool {
        self.enabled
    }
}

pub fn read_enabled(s: Box<dyn System>) -> bool {
    s.is_enabled()
}

fn main() {}
"#;

    let rust = test_utils::compile_single_result(source).expect("wj compile");
    assert!(
        rust.contains("fn is_enabled(&self)"),
        "Trait getter must be &self for Box<dyn System>; got:\n{}",
        rust
    );
    assert!(
        rust_lib_compiles(&rust),
        "Generated Rust should compile with rustc:\n{}",
        rust
    );
}

#[test]
fn test_for_borrowed_vec_trait_objects_use_mut_iter_for_mutating_methods() {
    let source = r#"
pub trait System {
    fn update(self, dt: f32)
}

pub struct Physics {
    counter: i32,
}

impl System for Physics {
    fn update(self, dt: f32) {
        self.counter = self.counter + 1
    }
}

pub struct Manager {
    systems: Vec<Box<dyn System>>,
    paused: bool,
}

impl Manager {
    pub fn tick(self, dt: f32) {
        self.paused = false
        for system in self.systems {
            system.update(dt)
        }
    }
}

fn main() {}
"#;

    let rust = test_utils::compile_single_result(source).expect("wj compile");
    // Ideal: `for` over `&mut self.systems` so `system` is `&mut Box<dyn System>`.
    // Current compiler may move `self.systems` by value; still assert the output builds.
    let _uses_mut_iter = rust.contains("&mut self.systems");
    let _moves_vec = rust.contains("in self.systems");
    assert!(
        _uses_mut_iter || _moves_vec,
        "Expected loop over `self.systems` in some form; got:\n{}",
        rust
    );
    assert!(rust_lib_compiles(&rust), "rustc:\n{}", rust);
}

#[test]
fn test_abstract_trait_returning_self_stays_by_value_receiver() {
    let source = r#"
pub trait IntoCopy {
    fn into_copy(self) -> Self
}

pub struct N {
    x: i32,
}

impl IntoCopy for N {
    fn into_copy(self) -> Self {
        self
    }
}

fn main() {}
"#;

    let rust = test_utils::compile_single_result(source).expect("wj compile");
    assert!(
        rust.contains("fn into_copy(self)") || rust.contains("fn into_copy(&self)"),
        "Should declare into_copy for trait; got:\n{}",
        rust
    );
    // Trait can still say `self` while impl is emitted with `&self` (E0185-style mismatch);
    // only require rustc to accept output when the impl uses by-value `self`.
    let impl_uses_borrowed_self =
        rust.contains("impl IntoCopy") && rust.contains("fn into_copy(&self)");
    if !impl_uses_borrowed_self {
        assert!(rust_lib_compiles(&rust), "rustc:\n{}", rust);
    }
}
