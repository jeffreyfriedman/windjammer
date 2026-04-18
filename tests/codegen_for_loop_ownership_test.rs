/// TDD test for for-loop ownership inference
/// 
/// Bug: For-loops always borrow elements (&T) even when methods need owned (T).
/// 
/// Example that should work:
/// ```windjammer
/// for item in items {
///     item.consume_self()  // Method needs `self` (owned)
/// }
/// ```
/// 
/// Current behavior: Compiler infers `&item`, method call fails with E0507
/// Expected behavior: Compiler should infer owned `item` when method requires it

use std::fs;
use std::process::Command;

#[test]
fn test_for_loop_infers_owned_when_method_consumes() {
    let source = r#"
enum Cond {
    HasGold(i32),
    HasItem(string),
}

impl Cond {
    pub fn check(self) -> bool {
        match self {
            Cond::HasGold(amount) => amount > 0,
            Cond::HasItem(_name) => false,
        }
    }
}

struct Node {
    pub conditions: Vec<Cond>,
}

impl Node {
    pub fn all_pass(self) -> bool {
        for cond in self.conditions {
            if !cond.check() {
                return false
            }
        }
        true
    }
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_for_loop_owned_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj compiler");

    let out_dir = test_dir.join("build");
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    // The key assertion: Method should consume self (owned) not borrow
    // Because it iterates over self.conditions and calls consuming methods
    assert!(
        generated.contains("pub fn all_pass(self) -> bool"),
        "Method all_pass should be inferred as consuming self (owned), not borrowing.\nGenerated:\n{}",
        generated
    );

    // For-loop should consume, not borrow
    assert!(
        generated.contains("for cond in self.conditions {")
            && !generated.contains("for cond in &self.conditions"),
        "For-loop should consume self.conditions (not borrow) when elements are consumed.\nGenerated:\n{}",
        generated
    );

    // Compile with rustc
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test.rlib"))
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

#[test]
fn test_for_loop_with_mutable_methods() {
    let source = r#"
struct Counter {
    pub value: i32,
}

impl Counter {
    pub fn increment(self) {
        self.value = self.value + 1
    }
}

fn increment_all(counters: Vec<Counter>) {
    for counter in counters {
        counter.increment()  // increment mutates self
    }
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_for_loop_mut_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let _output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj compiler");

    let out_dir = test_dir.join("build");
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    // Should compile successfully
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test.rlib"))
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
