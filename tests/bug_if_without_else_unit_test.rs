/// TDD Test: if-without-else blocks should not return values
///
/// Bug: When an if-block without else contains an expression that returns
/// a non-() value (like HashSet::insert() returning bool), Rust expects
/// the if-block to return () since there's no else branch.
///
/// The codegen should ensure the last expression in an if-without-else
/// block has a semicolon to suppress the return value.
use std::fs;
use std::process::Command;

#[test]
fn test_if_without_else_suppresses_return() {
    let source = r#"
use std::collections::HashSet

fn count_unique(items: Vec<i32>) -> i32 {
    let mut seen = HashSet::new()
    for item in &items {
        if *item > 0 {
            seen.insert(*item)
        }
    }
    seen.len() as i32
}

fn main() {
    let items = vec![1, 2, 3, 2, 1]
    let count = count_unique(items)
}
"#;

    let temp_dir = std::env::temp_dir();
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

    let _output = Command::new("wj")
        .arg("build")
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

    fs::remove_dir_all(&test_dir).ok();
}
