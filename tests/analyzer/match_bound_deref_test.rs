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

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_match_bound_var_no_deref_in_arithmetic() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");

    fs::write(
        &test_file,
        r#"
enum Shape {
    Point,
    Sphere(f32),
}

struct Emitter {
    shape: Shape,
}

impl Emitter {
    pub fn test(self) -> f32 {
        match self.shape {
            Shape::Point => 0.0,
            Shape::Sphere(radius) => {
                let x = 2.0 * radius
                let y = radius + 1.0
                x + y
            },
        }
    }
}
"#,
    )
    .unwrap();

    let output = Command::cargo_bin("wj")
        .unwrap()
        .arg("build")
        .arg(test_file.to_str().unwrap())
        .arg("--output")
        .arg(temp_dir.path().join("output").to_str().unwrap())
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    assert!(output.status.success(), "wj build failed: {:?}", output);

    let generated = fs::read_to_string(temp_dir.path().join("output/test.rs"))
        .expect("Failed to read generated file");

    assert!(
        !generated.contains("*radius"),
        "Match-bound Copy variable should NOT get * deref in arithmetic. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("2.0_f32 * radius"),
        "Should generate clean multiplication without deref. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("radius + 1.0_f32"),
        "Should generate clean addition without deref. Generated:\n{}",
        generated
    );
}
