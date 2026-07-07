#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD Test: Field assignment from &String parameter to String field
///
/// Problem: self.data.field = data where data: &String, field: String
/// Error: expected `String`, found `&String`
///
/// Solution: Auto-insert .clone() or .to_string() when assigning borrowed to owned field
use std::fs;
use std::process::Command;

#[test]
fn test_assign_borrowed_to_owned_field() {
    let source = r#"
struct Data {
    value: string
}

struct Manager {
    data: Data
}

impl Manager {
    pub fn set_value(self, value: string) {
        self.data.value = value
    }
}

fn main() {
    let mut mgr = Manager { data: Data { value: "initial" } }
    mgr.set_value("updated")
    println("{}", mgr.data.value)
}
"#;

    let _tmp = tempfile::tempdir().unwrap();

    let temp_dir = _tmp.path();

    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let _output = Command::new(wj_binary)
        .arg("build")
        .arg("--no-cargo")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, generated
        );
    }

    // When assigning param to owned field, param may be owned (direct move)
    // or borrowed (with clone). Both are correct Rust.
    assert!(
        generated.contains("value: &String")
            || generated.contains("value: String")
            || generated.contains("value: &str"),
        "Should generate a valid string parameter type. Got:\n{}",
        generated
    );
    // If borrowed, must clone; if owned, direct assignment is fine
    if generated.contains("value: &String") || generated.contains("value: &str") {
        assert!(
            generated.contains("value.clone()")
                || generated.contains("value.to_string()")
                || generated.contains("value.into()"),
            "Borrowed param assigned to owned field should clone/convert. Got:\n{}",
            generated
        );
    } else {
        assert!(
            generated.contains("self.data.value = value"),
            "Owned param should directly assign to field. Got:\n{}",
            generated
        );
    }
}
