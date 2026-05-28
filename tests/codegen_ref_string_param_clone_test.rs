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

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ref_string_param_to_owned_string() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_ref_string_param_clone");

    fs::create_dir_all(&test_dir).unwrap();

    // Test that iterator variables (&String from for loops) are auto-cloned
    // when passed to functions expecting String (owned)
    // This matches the pattern: for item in vec { Constructor::new(item) }
    let test_content = r#"
struct Member {
    name: string,
}

impl Member {
    fn new(name: string) -> Member {
        Member { name }
    }
}

fn create_members(names: &Vec<string>) -> Vec<Member> {
    let mut members = Vec::new();
    for name in names {
        members.push(Member::new(name));
    }
    members
}

fn main() {
    let names = vec!["Alice".to_string(), "Bob".to_string()];
    let members = create_members(&names);
}
"#;

    let test_file = test_dir.join("ref_string_param_clone.wj");
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

    let rust_file = test_dir.join("build").join("ref_string_param_clone.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should auto-convert iterator variables for owned parameters.
    // for name in names → name is &String, Member::new expects String.
    // Windjammer text types use .to_string() (not .clone()) because when the
    // param is lowered to &str, .clone() keeps &str (Copy), not String.
    assert!(
        rust_code.contains("Member::new(name.to_string())")
            || rust_code.contains("Member::new(name.clone())"),
        "Expected auto-conversion for iterator variable (&String -> String).\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles
    let compile_output = Command::new("rustc")
        .current_dir(test_dir.join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg("ref_string_param_clone.rs")
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
