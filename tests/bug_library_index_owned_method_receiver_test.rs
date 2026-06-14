#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Library multipass (`--library --module-file`) must match single-file index+owned-method clone fix.
#[test]
fn test_library_module_index_owned_method_receiver_clones() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let mod_dir = src.join("nodes");
    fs::create_dir_all(&mod_dir).expect("mkdir");

    fs::write(
        mod_dir.join("mod.wj"),
        r#"pub mod graph;
"#,
    )
    .unwrap();

    fs::write(
        mod_dir.join("graph.wj"),
        r##"
pub enum Value {
    Float(f32),
    Text(string),
    None,
}

impl Value {
    pub fn as_float(self) -> f32 {
        match self {
            Value::Float(v) => v,
            _ => 0.0,
        }
    }
}

pub struct Node {
    inputs: Vec<Value>,
}

impl Node {
    pub fn read_input(self) -> f32 {
        self.inputs[0].as_float()
    }

    pub fn get_output(self, index: usize) -> Value {
        self.inputs[index]
    }
}
"##,
    )
    .unwrap();

    let out = tmp.path().join("gen");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
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
        output.status.success(),
        "library build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(out.join("nodes/graph.rs")).unwrap_or_else(|_| {
        fs::read_to_string(out.join("nodes/nodes/graph.rs")).expect("graph.rs output")
    });

    println!("Generated:\n{}", generated);

    assert!(
        generated.contains(".clone().as_float()"),
        "library module must clone before owned method on indexed non-Copy. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("inputs[index]") && generated.contains(".clone()"),
        "library module must clone indexed non-Copy return. Got:\n{}",
        generated
    );
}
