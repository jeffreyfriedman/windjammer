/// TDD: String interpolation is the idiomatic Windjammer way to build strings.
/// format!() is Rust leakage and should emit a deprecation warning.

use std::fs;
use std::process::Command;

fn find_rs_file(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    if !dir.exists() {
        return None;
    }
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "rs") {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_rs_file(&path) {
                return Some(found);
            }
        }
    }
    None
}

fn compile_wj(source: &str) -> (String, String, String) {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_interp_{}_{}",
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

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let generated = if output.status.success() {
        find_rs_file(&out_dir)
            .and_then(|p| fs::read_to_string(&p).ok())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let _ = fs::remove_dir_all(&test_dir);

    (generated, stdout, stderr)
}

#[test]
fn test_string_interpolation_compiles_to_format() {
    let source = r#"
pub fn greet(name: String) -> String {
    "Hello, ${name}!"
}
"#;

    let (generated, _stdout, _stderr) = compile_wj(source);
    assert!(
        !generated.is_empty(),
        "String interpolation should compile successfully"
    );
    assert!(
        generated.contains("format!"),
        "String interpolation should lower to format! in Rust codegen. Got:\n{}",
        generated
    );
}

#[test]
fn test_format_macro_emits_deprecation_warning() {
    let source = r#"
pub fn greet(name: String) -> String {
    format!("Hello, {}", name)
}
"#;

    let (_generated, _stdout, stderr) = compile_wj(source);
    assert!(
        stderr.contains("format!() is Rust syntax"),
        "format!() should emit a deprecation warning. Stderr:\n{}",
        stderr
    );
    assert!(
        stderr.contains("string interpolation"),
        "Warning should suggest string interpolation. Stderr:\n{}",
        stderr
    );
}

#[test]
fn test_interpolation_no_warning() {
    let source = r#"
pub fn greet(name: String) -> String {
    "Hello, ${name}!"
}
"#;

    let (_generated, _stdout, stderr) = compile_wj(source);
    assert!(
        !stderr.contains("format!() is Rust syntax"),
        "String interpolation should NOT emit format deprecation warning. Stderr:\n{}",
        stderr
    );
}

#[test]
fn test_interpolation_with_expression() {
    let source = r#"
pub fn describe(x: i32, y: i32) -> String {
    "Point(${x}, ${y})"
}
"#;

    let (generated, _stdout, _stderr) = compile_wj(source);
    assert!(
        !generated.is_empty(),
        "Interpolation with multiple expressions should compile"
    );
    assert!(
        generated.contains("format!"),
        "Should lower to format! macro"
    );
}

#[test]
fn test_interpolation_with_field_access() {
    let source = r#"
pub struct Player {
    pub name: String,
    pub score: i32,
}

pub fn status(p: Player) -> String {
    "${p.name}: ${p.score} points"
}
"#;

    let (generated, _stdout, _stderr) = compile_wj(source);
    assert!(
        !generated.is_empty(),
        "Interpolation with field access should compile"
    );
}
