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
))]
//! A by-value `self` method called on a collection element (`v[i].m()`, `self.items[i].m()`) must not move
//! the element out of its collection: rustc rejects that (E0507) whether or not the collection is behind
//! a reference, so the element is cloned at the call.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn type_checks(src: &str) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("m.wj"), src).unwrap();
    let o = Command::new(WJ).args(["build", "m.wj", "--no-cargo", "--output", "out"]).current_dir(dir.path()).output().unwrap();
    assert!(o.status.success(), "wj build failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let rs = dir.path().join("out/m.rs");
    let r = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "-o"])
        .arg(dir.path().join("o.rmeta"))
        .arg(&rs)
        .output()
        .unwrap();
    assert!(r.status.success(), "generated Rust does not type-check:\n{}\n--- generated\n{}", String::from_utf8_lossy(&r.stderr), fs::read_to_string(&rs).unwrap_or_default());
}

const VALUE: &str = "pub enum Value {\n    Int(i32),\n    Float(f32),\n    Text(string),\n}\n\nimpl Value {\n    pub fn as_float(self) -> f32 {\n        match self {\n            Value::Float(v) => v,\n            Value::Int(v) => v as f32,\n            _ => 0.0,\n        }\n    }\n}\n";

/// The owning method (`self`) reads its own element.
#[test]
fn by_value_self_method_on_an_element_of_an_owned_self_field() {
    type_checks(&format!("{VALUE}\npub struct Node {{\n    pub inputs: Vec<Value>,\n}}\n\nimpl Node {{\n    pub fn add(self) -> f32 {{\n        let a = self.inputs[0].as_float()\n        let b = self.inputs[1].as_float()\n        a + b\n    }}\n}}\n"));
}

/// A borrowing method reads an element of a borrowed field.
#[test]
fn by_value_self_method_on_an_element_of_a_borrowed_field() {
    type_checks(&format!("{VALUE}\npub struct Node {{\n    pub inputs: Vec<Value>,\n}}\n\nimpl Node {{\n    pub fn first(self) -> f32 {{\n        self.inputs[0].as_float()\n    }}\n}}\n"));
}

/// A local owned vector.
#[test]
fn by_value_self_method_on_an_element_of_a_local_vec() {
    type_checks(&format!("{VALUE}\npub fn total(vals: Vec<Value>) -> f32 {{\n    let x = vals[0].as_float()\n    x + vals[1].as_float()\n}}\n"));
}
