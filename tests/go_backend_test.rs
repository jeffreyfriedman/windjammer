/// Integration tests for the Go backend
///
/// Verifies that the Go backend generates valid, executable Go code
/// that produces the same output as the Rust backend.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Returns true if Go is available on this machine.
fn go_is_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile .wj source to Go and return the generated Go code
fn compile_to_go(source: &str) -> String {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("go")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Go compilation failed:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    let generated_file = output_dir.join("main.go");
    fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated Go file. Compiler stderr:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    })
}

/// Compile .wj to Go, build with `go build`, and run the binary.
/// Returns `Some(stdout)` on success, or `None` if `go` is not installed.
fn compile_and_run_go(source: &str) -> Option<String> {
    if !go_is_available() {
        eprintln!("Skipping: `go` not found in PATH");
        return None;
    }

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    // Compile .wj → Go
    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("go")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Go codegen failed:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    // Run with `go run`
    let go_output = Command::new("go")
        .arg("run")
        .arg("main.go")
        .current_dir(&output_dir)
        .output()
        .expect("Failed to execute go run");

    if !go_output.status.success() {
        let generated = fs::read_to_string(output_dir.join("main.go")).unwrap_or_default();
        panic!(
            "Go run failed:\n{}\n\nGenerated code:\n{}",
            String::from_utf8_lossy(&go_output.stderr),
            generated
        );
    }

    Some(String::from_utf8(go_output.stdout).unwrap())
}

// ==========================================
// Basic tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_hello_world() {
    let Some(output) = compile_and_run_go(
        r#"
fn main() {
    println("Hello, world!")
}
"#,
    ) else { return; };
    assert_eq!(output.trim(), "Hello, world!");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_arithmetic() {
    let Some(output) = compile_and_run_go(
        r#"
fn main() {
    let a = 1 + 2
    println("{}", a)
    let b = 10 - 3
    println("{}", b)
    let c = 6 * 7
    println("{}", c)
}
"#,
    ) else { return; };
    assert_eq!(output.trim(), "3\n7\n42");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_control_flow() {
    let Some(output) = compile_and_run_go(
        r#"
fn main() {
    let x = 5
    if x > 3 {
        println("big")
    } else {
        println("small")
    }

    let mut i = 0
    while i < 3 {
        println("{}", i)
        i += 1
    }
}
"#,
    ) else { return; };
    assert_eq!(output.trim(), "big\n0\n1\n2");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_functions() {
    let Some(output) = compile_and_run_go(
        r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(10, 20)
    println("{}", result)
}
"#,
    ) else { return; };
    assert_eq!(output.trim(), "30");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_for_range() {
    let Some(output) = compile_and_run_go(
        r#"
fn main() {
    for i in 0..3 {
        println("{}", i)
    }
}
"#,
    ) else { return; };
    assert_eq!(output.trim(), "0\n1\n2");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_struct() {
    let Some(output) = compile_and_run_go(
        r#"
struct Point {
    x: int,
    y: int
}

impl Point {
    fn sum(self) -> int {
        self.x + self.y
    }
}

fn main() {
    let p = Point { x: 3, y: 4 }
    println("{}", p.sum())
}
"#,
    ) else { return; };
    assert_eq!(output.trim(), "7");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_generates_package_main() {
    let code = compile_to_go(
        r#"
fn main() {
    println("test")
}
"#,
    );
    assert!(code.contains("package main"));
    assert!(code.contains("import \"fmt\""));
    assert!(code.contains("func main()"));
}
