// TDD Test: Method Calls Should Use Dot Syntax, Not Module Syntax
// Bug: When a variable has the same name as a common module (json, fmt, etc.),
//      method calls are incorrectly generated as module calls (::) instead of dot (.)
// Root Cause: Code generator doesn't distinguish between variable.method() and module::function()
// Fix: Always use dot syntax for method calls on local variables

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_calls_use_dot_not_colon_colon() {
    // Create a temp directory
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");

    // Write Windjammer code with a variable named "json" (common name)
    fs::write(
        &test_file,
        r#"fn test_json_variable() {
    let mut json = String::from("{")
    json.push_str("test")
    json.push_str("}")
    json
}

fn test_other_variable() {
    let mut result = String::from("hello")
    result.push_str(" world")
    result
}
"#,
    )
    .unwrap();

    // Run wj build with locally built binary
    let wj_binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wj");

    let output = Command::new(&wj_binary)
        .args(["build", test_file.to_str().unwrap(), "--no-cargo"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("wj build failed");
    }

    // Read the generated Rust code
    let rust_file = temp_dir.path().join("build/test.rs");
    let rust_code = fs::read_to_string(&rust_file).expect("Failed to read generated Rust");

    println!("Generated Rust code:\n{}", rust_code);

    // THE TEST: Verify method calls use dot syntax, not module syntax
    assert!(
        !rust_code.contains("json::push_str"),
        "Generated code should NOT use json::push_str (module syntax). \
         It should use json.push_str (method call syntax). \
         Found in:\n{}",
        rust_code
    );

    assert!(
        !rust_code.contains("result::push_str"),
        "Generated code should NOT use result::push_str (module syntax)"
    );

    // Verify the correct syntax is generated
    assert!(
        rust_code.contains("json.push_str"),
        "Generated code should use json.push_str (method call syntax)"
    );

    assert!(
        rust_code.contains("result.push_str"),
        "Generated code should use result.push_str (method call syntax)"
    );

    // Verify the generated Rust compiles
    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&rust_file)
        .arg("--out-dir")
        .arg(temp_dir.path().join("build"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        eprintln!(
            "Rustc STDERR:\n{}",
            String::from_utf8_lossy(&rustc_output.stderr)
        );
        panic!("Generated Rust code does not compile!");
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_calls_on_self_fields() {
    // Create a temp directory
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");

    // Write Windjammer code with method calls on self fields
    fs::write(
        &test_file,
        r#"pub struct Builder {
    pub json: string,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            json: String::from(""),
        }
    }

    pub fn add_field(field: string) {
        self.json.push_str(field)
    }
}
"#,
    )
    .unwrap();

    // Run wj build with locally built binary
    let wj_binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wj");

    let output = Command::new(&wj_binary)
        .args(["build", test_file.to_str().unwrap(), "--no-cargo"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("wj build failed");
    }

    // Read the generated Rust code
    let rust_file = temp_dir.path().join("build/test.rs");
    let rust_code = fs::read_to_string(&rust_file).expect("Failed to read generated Rust");

    println!("Generated Rust code:\n{}", rust_code);

    // THE TEST: self.field.method() should stay as-is
    assert!(
        !rust_code.contains("self.json::push_str") && !rust_code.contains("self::json::push_str"),
        "self.json.push_str should NOT be changed to module syntax"
    );

    assert!(
        rust_code.contains("self.json.push_str"),
        "self.json.push_str should be preserved correctly"
    );
}
