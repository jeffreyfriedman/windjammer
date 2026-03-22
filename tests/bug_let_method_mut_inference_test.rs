use std::fs;
/// TDD test: Mutable method calls in let bindings should trigger &mut inference
///
/// Bug: `let x = loader.load(...)` where `load()` requires `&mut self`
/// doesn't trigger &mut inference for `loader` parameter because `is_mutated`
/// only checks `Statement::Expression`, not `Statement::Let` values.
///
/// Root Cause: `is_mutated` doesn't check the value expression of let bindings
/// for mutable method calls.
///
/// Fix: Add `Statement::Let` case in `is_mutated` to check value for
/// mutable method calls.
use std::process::Command;

fn transpile_wj(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Compilation failed:\nSTDERR:\n{}\nSTDOUT:\n{}",
            stderr, stdout
        );
    }

    let rust_file = out_dir.join("test.rs");
    let content = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    let _ = fs::remove_dir_all(&test_dir);

    content
}

#[test]
fn test_let_binding_with_mut_method_call() {
    let source = r#"
pub struct Loader {
    count: i32,
    items: Vec<String>,
}

impl Loader {
    pub fn new() -> Loader {
        Loader { count: 0, items: Vec::new() }
    }

    pub fn load(self, name: String, size: i32) -> String {
        self.count = self.count + 1
        self.items.push(name.clone())
        name
    }
}

pub fn load_stuff(loader: Loader) -> Vec<String> {
    let mut results: Vec<String> = Vec::new()
    let a = loader.load("first".to_string(), 100)
    let b = loader.load("second".to_string(), 200)
    results.push(a)
    results.push(b)
    results
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // THE WINDJAMMER WAY: Automatic ownership inference!
    // User writes `loader: Loader` (no & or &mut)
    // Compiler infers `loader: &mut Loader` because loader.load() mutates (self.count++)
    // This is automatic ownership inference - compiler does the hard work!
    assert!(
        generated.contains("loader: &mut Loader"),
        "Parameter should be inferred as `&mut Loader` (automatic ownership). Got:\n{}",
        generated
    );
}
