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

use std::process::Command;

fn compile_wj(code: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let wj_path = dir.path().join("test.wj");
    std::fs::write(&wj_path, code).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("--library")
        .arg("test.wj")
        .current_dir(dir.path())
        .output()
        .expect("failed to run wj compiler");

    if !output.status.success() {
        panic!(
            "wj build failed. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let rs_path = dir.path().join("build/test.rs");
    std::fs::read_to_string(&rs_path).unwrap_or_else(|_| {
        panic!(
            "No .rs output at {}. stderr: {}",
            rs_path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    })
}

/// Bug: String literals passed to chained method calls don't get .to_string() conversion.
/// Scenario::new("test".to_string()) works, but .spawn_entity("enemy", ...) doesn't.
#[test]
fn test_string_literal_in_chained_method_call() {
    let code = r#"
pub struct Builder {
    pub name: string,
    pub items: Vec<string>,
}

impl Builder {
    pub fn new(name: string) -> Builder {
        Builder { name: name, items: Vec::new() }
    }

    pub fn add_item(self, item: string) -> Builder {
        self.items.push(item)
        self
    }

    pub fn with_label(self, label: string) -> Builder {
        self.name = label
        self
    }
}

pub fn test_chain() {
    let b = Builder::new("test")
        .add_item("alpha")
        .add_item("beta")
        .with_label("renamed")
}
"#;

    let output = compile_wj(code);

    // All string literals in chained calls should get .to_string()
    assert!(
        output.contains(r#""test".to_string()"#),
        "String literal 'test' should get .to_string() in new(): {}",
        output
    );
    assert!(
        output.contains(r#""alpha".to_string()"#),
        "String literal 'alpha' should get .to_string() in chained add_item(): {}",
        output
    );
    assert!(
        output.contains(r#""beta".to_string()"#),
        "String literal 'beta' should get .to_string() in second chained add_item(): {}",
        output
    );
    assert!(
        output.contains(r#""renamed".to_string()"#),
        "String literal 'renamed' should get .to_string() in chained with_label(): {}",
        output
    );
}

#[test]
fn test_string_literal_in_method_with_multiple_params() {
    let code = r#"
pub struct Config {
    pub entries: Vec<string>,
}

impl Config {
    pub fn new() -> Config {
        Config { entries: Vec::new() }
    }

    pub fn set(self, key: string, x: f32, y: f32, z: f32) -> Config {
        self.entries.push(key)
        self
    }
}

pub fn test_multi_param() {
    let c = Config::new()
        .set("position", 1.0, 2.0, 3.0)
        .set("rotation", 0.0, 90.0, 0.0)
}
"#;

    let output = compile_wj(code);

    assert!(
        output.contains(r#""position".to_string()"#),
        "String literal 'position' should get .to_string(): {}",
        output
    );
    assert!(
        output.contains(r#""rotation".to_string()"#),
        "String literal 'rotation' should get .to_string(): {}",
        output
    );
}
