// TDD: Test that compiler adds #[allow(dead_code)] to unused fields
// Write tests FIRST, then implement usage tracking

use std::fs;
use tempfile::TempDir;

#[test]
fn test_unused_struct_field_gets_allow_attribute() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.wj");

    // Struct with unused field
    fs::write(
        &input_file,
        r#"
pub struct MyStruct {
    used_field: int,
    unused_field: int,
}

pub fn use_struct(s: MyStruct) {
    println!("{}", s.used_field);
}
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&input_file)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    let rust_code = fs::read_to_string(output_dir.join("input.rs")).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // Try to compile with -D warnings (warnings become errors)
    let compile_result = std::process::Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(output_dir.join("input.rs"))
        .arg("--edition")
        .arg("2021")
        .arg("-D")
        .arg("warnings")
        .current_dir(&output_dir)
        .output();

    if let Ok(output) = compile_result {
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success()
            && (stderr.contains("dead_code")
                || stderr.contains("never read")
                || stderr.contains("unused"))
        {
            panic!(
                "Compiler should add #[allow(dead_code)] to unused fields:\n{}",
                stderr
            );
        }
    }
}

#[test]
fn test_all_fields_used_no_allow_needed() {
    // Baseline: when all fields are used, no #[allow(dead_code)] needed
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.wj");

    fs::write(
        &input_file,
        r#"
pub struct Point {
    x: int,
    y: int,
}

pub fn distance(p: Point) -> int {
    p.x * p.x + p.y * p.y
}
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&input_file)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Should compile fine without any special attributes
    let compile_result = std::process::Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(output_dir.join("input.rs"))
        .arg("--edition")
        .arg("2021")
        .arg("-D")
        .arg("warnings")
        .current_dir(&output_dir)
        .output();

    if let Ok(output) = compile_result {
        assert!(
            output.status.success(),
            "Code with all fields used should compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
