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

/// TDD test for for-loop ownership inference
///
/// Pattern: `match self { Enum::Variant(v) => v > 0 }` — Rust's match ergonomics
/// allow matching on `&self` with auto-deref, so these methods don't need to consume.
/// The compiler should infer `&self` for both the enum method and the iterating method,
/// producing borrowed iteration that compiles correctly.
use std::fs;
use std::process::Command;

#[test]
fn test_for_loop_match_self_enum_borrows_correctly() {
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

    let _tmp = tempfile::tempdir().unwrap();

    let temp_dir = _tmp.path();

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

    // check() only reads fields via match — Rust match ergonomics handles &self
    assert!(
        generated.contains("fn check(&self)"),
        "check should be &self (match on enum doesn't require consuming), got:\n{}",
        generated
    );

    // all_pass should borrow since check doesn't consume
    assert!(
        generated.contains("fn all_pass(&self)"),
        "all_pass should be &self since check is &self, got:\n{}",
        generated
    );

    // For-loop should borrow
    assert!(
        generated.contains("for cond in &self.conditions"),
        "For-loop should borrow self.conditions, got:\n{}",
        generated
    );

    // Compile with rustc — the critical check
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

    let _tmp2 = tempfile::tempdir().unwrap();

    let temp_dir = _tmp2.path();

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
}
