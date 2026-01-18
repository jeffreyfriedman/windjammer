use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_auto_mut_on_compound_assignment() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_auto_mut_compound");
    
    fs::create_dir_all(&test_dir).unwrap();

    // Test that compound assignments trigger auto-mutability
    let test_content = r#"
fn count_items(items: Vec<i32>) -> i32 {
    let total = 0;
    for item in items {
        total += item;
    }
    total
}

fn main() {
    let result = count_items(vec![1, 2, 3]);
}
"#;

    let test_file = test_dir.join("compound.wj");
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

    // Check the generated Rust code
    let rust_file = test_dir.join("build").join("compound.rs");
    assert!(
        rust_file.exists(),
        "Expected generated Rust file to exist at {:?}",
        rust_file
    );

    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should have `let mut total = 0;`
    assert!(
        rust_code.contains("let mut total = 0;"),
        "Expected auto-mutability to add 'mut' for compound assignment.\nGenerated code:\n{}",
        rust_code
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}

