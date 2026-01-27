use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_ref_string_param_auto_clone_to_owned() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_ref_string_clone");

    fs::create_dir_all(&test_dir).unwrap();

    // Test that &string parameters are auto-cloned when passed to functions expecting string
    // This matches the pattern: fn add_member(role: &string) { ... Member::new(role) }
    // where Member::new expects owned string
    let test_content = r#"
struct Member {
    role: string,
}

impl Member {
    fn new(role: string) -> Member {
        Member { role }
    }
}

fn create_member(role: &string) -> Member {
    Member::new(role)
}

fn main() {
    let role = "Warrior".to_string();
    let member = create_member(&role);
}
"#;

    let test_file = test_dir.join("ref_string_clone.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg(&test_file)
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.join("build").join("ref_string_clone.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should auto-convert: Member::new(role.to_string())
    // (&str -> String requires .to_string(), not .clone())
    assert!(
        rust_code.contains("Member::new(role.to_string())"),
        "Expected auto-convert for &str -> String.\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles
    let compile_output = Command::new("rustc")
        .current_dir(test_dir.join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg("ref_string_clone.rs")
        .output()
        .expect("Failed to run rustc");

    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    assert!(
        compile_output.status.success(),
        "Expected generated code to compile.\nRustc errors:\n{}",
        compile_stderr
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}
