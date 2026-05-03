/// TDD: Compiler should not panic on duplicate function names in impl blocks.
///
/// Bug: When an impl block contains two functions with the same name, the
/// analyzer's fixed-point iteration panics because `analyzed_funcs.remove()`
/// succeeds for the first occurrence but fails for the second.
///
/// Fix: Skip duplicate occurrences gracefully instead of panicking.
/// The first definition wins.

use std::process::Command;

fn compile_wj(source: &str) -> (bool, String) {
    let dir = tempfile::tempdir().unwrap();
    let src_path = dir.path().join("test.wj");
    std::fs::write(&src_path, source).unwrap();

    let wj = std::env::var("WJ_BINARY")
        .unwrap_or_else(|_| {
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            manifest.join("target/release/wj").to_string_lossy().to_string()
        });

    let output = Command::new(&wj)
        .arg("build")
        .arg(src_path.to_str().unwrap())
        .arg("--no-cargo")
        .arg("-o")
        .arg(dir.path().join("out").to_str().unwrap())
        .output()
        .expect("Failed to execute wj");

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    (output.status.success(), combined)
}

#[test]
fn test_duplicate_fn_in_impl_does_not_panic() {
    let source = r#"
pub struct Counter {
    pub value: i32,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { value: 0 }
    }

    pub fn count(self) -> i32 {
        self.value
    }

    // Duplicate -- same name, different body
    pub fn count(self) -> i32 {
        self.value + 1
    }
}
"#;

    let (success, output) = compile_wj(source);
    // Should not panic. May or may not succeed (could emit an error),
    // but must not crash the compiler.
    assert!(
        !output.contains("panicked"),
        "Compiler panicked on duplicate fn: {}",
        output
    );
    // The first definition should win, so it should compile
    assert!(success, "Expected compilation to succeed (first definition wins): {}", output);
}

#[test]
fn test_single_fn_in_impl_still_works() {
    let source = r#"
pub struct Adder {
    pub x: i32,
}

impl Adder {
    pub fn add(self, n: i32) -> i32 {
        self.x + n
    }
}
"#;

    let (success, output) = compile_wj(source);
    assert!(success, "Normal impl should compile: {}", output);
}
