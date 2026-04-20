/// TDD Test: Enum equality operator (==) usage in Windjammer code
///
/// Verifies that `a == b` and `a != b` work on unit enums and produce
/// valid Rust that compiles with rustc. The compiler derives PartialEq
/// for unit enums, so == should just work.
///
/// This test validates the entire pipeline: WJ source → generated Rust → rustc.
use std::process::Command;

fn compile_and_verify_rust(source: &str) -> (String, bool) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(".tmpEnumEq_{}_{}", std::process::id(), id));
    let _ = std::fs::create_dir_all(&dir);

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let build_dir = dir.join("build");
    let _ = std::fs::create_dir_all(&build_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            "--target",
            "rust",
            wj_file.to_str().unwrap(),
            "--output",
            build_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj");

    let rust_file = build_dir.join("test.rs");
    let content = std::fs::read_to_string(&rust_file).unwrap_or_default();

    if content.is_empty() {
        let _ = std::fs::remove_dir_all(&dir);
        return (String::from_utf8_lossy(&output.stderr).to_string(), false);
    }

    let rustc_output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg(&rust_file)
        .arg("-o")
        .arg(build_dir.join("test_lib"))
        .output()
        .expect("Failed to run rustc");

    let rustc_ok = rustc_output.status.success();
    let _ = std::fs::remove_dir_all(&dir);

    if !rustc_ok {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr).to_string();
        return (
            format!("Generated Rust:\n{}\n\nrustc errors:\n{}", content, stderr),
            false,
        );
    }

    (content, true)
}

#[test]
fn test_unit_enum_equality_operator() {
    let source = r#"
enum Direction {
    North,
    South,
    East,
    West,
}

pub fn is_north(d: Direction) -> bool {
    d == Direction::North
}

pub fn is_not_south(d: Direction) -> bool {
    d != Direction::South
}
"#;
    let (info, ok) = compile_and_verify_rust(source);
    assert!(ok, "Unit enum == should compile through rustc:\n{}", info);
}

#[test]
fn test_enum_equality_in_if_condition() {
    let source = r#"
enum Color {
    Red,
    Green,
    Blue,
}

pub fn color_name(c: Color) -> string {
    if c == Color::Red {
        "red"
    } else if c == Color::Green {
        "green"
    } else {
        "blue"
    }
}
"#;
    let (info, ok) = compile_and_verify_rust(source);
    assert!(ok, "Enum == in if conditions should compile:\n{}", info);
}

#[test]
fn test_enum_equality_with_variable() {
    let source = r#"
enum State {
    Active,
    Paused,
    Stopped,
}

pub fn states_equal(a: State, b: State) -> bool {
    a == b
}

pub fn states_not_equal(a: State, b: State) -> bool {
    a != b
}
"#;
    let (info, ok) = compile_and_verify_rust(source);
    assert!(ok, "Enum == between variables should compile:\n{}", info);
}
