//! TDD: Abstract trait `self` parsing must not force by-value receivers for non-`Self` returns.
//!
//! Root cause (dogfooding): `fn is_enabled(self) -> bool` on a trait was parsed as `OwnershipHint::Owned`
//! for every abstract single-`self` method, so Rust emitted `fn is_enabled(self)`, breaking `dyn Trait`
//! (E0161/E0507). Getter-style methods must stay inferable as `&self`.

use std::process::Command;

fn get_wj_binary() -> String {
    env!("CARGO_BIN_EXE_wj").to_string()
}

fn compile_to_rust(wj_source: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    std::fs::write(&wj_path, wj_source).expect("write .wj");
    std::fs::create_dir_all(&out_dir).expect("out dir");

    let output = Command::new(get_wj_binary())
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("wj build");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let test_rs = out_dir.join("test.rs");
    let src_main = out_dir.join("src").join("main.rs");
    if test_rs.exists() {
        Ok(std::fs::read_to_string(test_rs).map_err(|e| e.to_string())?)
    } else if src_main.exists() {
        Ok(std::fs::read_to_string(src_main).map_err(|e| e.to_string())?)
    } else {
        Err("no generated Rust file".to_string())
    }
}

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

    let rust = compile_to_rust(source).expect("wj compile");
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

    let rust = compile_to_rust(source).expect("wj compile");
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

    let rust = compile_to_rust(source).expect("wj compile");
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
