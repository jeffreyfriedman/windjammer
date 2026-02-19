/// TDD Test: if-let codegen optimization
///
/// Bug: The parser converts `if let` to a `Statement::Match` with 2 arms:
///   - Arm 0: the destructuring pattern (e.g., `Some(x)`) with body
///   - Arm 1: `Pattern::Wildcard` with an empty block body
/// The codegen then emits a full `match` statement with `_ => {}`, triggering
/// clippy's "you seem to be trying to use `match` for destructuring a single
/// pattern" warning (84 instances in windjammer-game).
///
/// Fix: Detect this pattern in Statement::Match codegen and emit `if let`
/// instead of `match`. When the wildcard arm has a non-empty body, emit
/// `if let ... { } else { }`.
use std::io::Write;
use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = dir.path().join("test.wj");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut file = std::fs::File::create(&wj_path).unwrap();
    file.write_all(source.as_bytes()).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "--no-cargo",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let rs_path = out_dir.join("test.rs");
    if rs_path.exists() {
        std::fs::read_to_string(&rs_path).unwrap()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "No output file generated.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
}

#[test]
fn test_if_let_some_generates_if_let() {
    // `if let Some(x) = expr { ... }` should generate `if let Some(x) = expr { ... }`
    // NOT `match expr { Some(x) => { ... }, _ => {} }`
    let source = r#"
pub struct Container {
    pub value: Option<i32>,
}

impl Container {
    pub fn process(self) {
        if let Some(v) = self.value {
            println("{}", v)
        }
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("if let Some(v)"),
        "if let Some(v) should generate `if let`, not `match`.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("match self.value"),
        "Should NOT generate a match statement for if-let pattern.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_if_let_with_else_generates_if_let_else() {
    // `if let Some(x) = expr { ... } else { ... }` should generate `if let ... else`
    let source = r#"
pub struct Container {
    pub value: Option<i32>,
}

impl Container {
    pub fn get_or_default(self) -> i32 {
        if let Some(v) = self.value {
            v
        } else {
            0
        }
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("if let Some(v)"),
        "if let with else should generate `if let`, not `match`.\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("} else {"),
        "if let with else should have an else block.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_if_let_with_mutable_ref() {
    // `if let Some(x) = map.get_mut(&key) { ... }` should also produce if let
    let source = r#"
use std::collections::HashMap

pub struct Registry {
    pub items: HashMap<String, i32>,
}

impl Registry {
    pub fn increment(self, key: String) {
        if let Some(val) = self.items.get_mut(&key) {
            *val = *val + 1
        }
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("if let Some(val)"),
        "if let on get_mut should generate `if let`.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_real_match_not_converted() {
    // A real match with multiple meaningful arms should NOT be converted to if-let
    let source = r#"
pub fn describe(opt: Option<i32>) -> String {
    match opt {
        Some(v) => format!("Value: {}", v),
        None => "Nothing".to_string(),
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("match opt"),
        "Real match with meaningful arms should stay as match.\nGenerated:\n{}",
        generated
    );
}
