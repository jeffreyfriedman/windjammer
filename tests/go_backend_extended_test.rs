/// Extended integration tests for Go backend (TDD)
///
/// Tests enum generation, match statements, trait→interface,
/// closures, variable shadowing, and constants.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

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
/// Returns stdout output.
fn compile_and_run_go(source: &str) -> String {
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

    String::from_utf8(go_output.stdout).unwrap()
}

// ==========================================
// Enum tests (tagged unions → interface + variant structs)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_enum_unit_variants() {
    // Enums with unit variants should become an interface + empty structs
    let code = compile_to_go(
        r#"
enum Color {
    Red,
    Green,
    Blue
}

fn main() {
    println("ok")
}
"#,
    );
    // Should generate an interface with a marker method
    assert!(
        code.contains("type Color interface"),
        "Should generate Color interface. Got:\n{}",
        code
    );
    assert!(
        code.contains("IsColor()"),
        "Should have marker method IsColor(). Got:\n{}",
        code
    );
    // Should generate variant structs
    assert!(
        code.contains("type ColorRed struct"),
        "Should generate ColorRed struct. Got:\n{}",
        code
    );
    assert!(
        code.contains("type ColorGreen struct"),
        "Should generate ColorGreen struct. Got:\n{}",
        code
    );
    assert!(
        code.contains("type ColorBlue struct"),
        "Should generate ColorBlue struct. Got:\n{}",
        code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_enum_tuple_variants() {
    // Enums with tuple data should become structs with Field0, Field1, etc.
    let code = compile_to_go(
        r#"
enum Shape {
    Circle(float),
    Rectangle(float, float)
}

fn main() {
    println("ok")
}
"#,
    );
    assert!(
        code.contains("type Shape interface"),
        "Should generate Shape interface. Got:\n{}",
        code
    );
    assert!(
        code.contains("type ShapeCircle struct"),
        "Should generate ShapeCircle. Got:\n{}",
        code
    );
    assert!(
        code.contains("Field0 float64"),
        "ShapeCircle should have Field0 float64. Got:\n{}",
        code
    );
    assert!(
        code.contains("type ShapeRectangle struct"),
        "Should generate ShapeRectangle. Got:\n{}",
        code
    );
}

// ==========================================
// Match / Switch tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_match_integer_values() {
    // Match on integer values should become a Go switch statement
    let output = compile_and_run_go(
        r#"
fn describe(x: int) -> string {
    match x {
        1 => "one",
        2 => "two",
        _ => "other"
    }
}

fn main() {
    println("{}", describe(1))
    println("{}", describe(2))
    println("{}", describe(99))
}
"#,
    );
    assert_eq!(output.trim(), "one\ntwo\nother");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_match_generates_switch() {
    // Verify the generated Go code uses switch
    let code = compile_to_go(
        r#"
fn check(x: int) -> int {
    match x {
        0 => 10,
        1 => 20,
        _ => 30
    }
}

fn main() {
    println("{}", check(0))
}
"#,
    );
    assert!(
        code.contains("switch"),
        "Match should generate switch. Got:\n{}",
        code
    );
    assert!(
        code.contains("case 0"),
        "Should have case 0. Got:\n{}",
        code
    );
    assert!(
        code.contains("default"),
        "Should have default case. Got:\n{}",
        code
    );
}

// ==========================================
// Trait → Interface tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_trait_generates_interface() {
    // Traits should become Go interfaces
    let code = compile_to_go(
        r#"
trait Drawable {
    fn draw(self)
    fn area(self) -> float
}

fn main() {
    println("ok")
}
"#,
    );
    assert!(
        code.contains("type Drawable interface"),
        "Trait should become interface. Got:\n{}",
        code
    );
    assert!(
        code.contains("Draw()"),
        "Should have Draw() method. Got:\n{}",
        code
    );
    assert!(
        code.contains("Area()"),
        "Should have Area() method. Got:\n{}",
        code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_trait_with_params() {
    // Trait methods with parameters should have typed params in the interface
    let code = compile_to_go(
        r#"
trait Resizable {
    fn resize(self, width: float, height: float) -> bool
}

fn main() {
    println("ok")
}
"#,
    );
    assert!(
        code.contains("type Resizable interface"),
        "Should generate Resizable interface. Got:\n{}",
        code
    );
    assert!(
        code.contains("Resize("),
        "Should have Resize method. Got:\n{}",
        code
    );
    assert!(
        code.contains("float64") && code.contains("bool"),
        "Should have float64 params and bool return. Got:\n{}",
        code
    );
}

// ==========================================
// Closure tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_closure_generates_func_literal() {
    // Closures should become Go function literals
    let code = compile_to_go(
        r#"
fn main() {
    let add = |a, b| a + b
    println("ok")
}
"#,
    );
    assert!(
        code.contains("func("),
        "Closure should generate func literal. Got:\n{}",
        code
    );
}

// ==========================================
// Variable shadowing tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_variable_shadowing() {
    // Variable shadowing should use `var` for re-declaration
    let output = compile_and_run_go(
        r#"
fn main() {
    let x = 10
    println("{}", x)
    let x = 20
    println("{}", x)
}
"#,
    );
    assert_eq!(output.trim(), "10\n20");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_shadowing_generates_reassignment() {
    // Second declaration of same variable should use `=` (assignment) to avoid Go's
    // "no new variables on left side of :=" error
    let code = compile_to_go(
        r#"
fn main() {
    let x = 5
    let x = 10
    println("{}", x)
}
"#,
    );
    // First declaration should use :=
    assert!(
        code.contains("x := 5"),
        "First decl should use :=. Got:\n{}",
        code
    );
    // Second declaration MUST NOT use := (Go error: no new variables)
    // It should use = (assignment) instead
    let after_first = code.split("x := 5").nth(1).unwrap_or("");
    assert!(
        !after_first.contains("x :="),
        "Second decl should NOT use :=. Got:\n{}",
        code
    );
    assert!(
        after_first.contains("x = 10"),
        "Second decl should use = (assignment). Got:\n{}",
        code
    );
}

// ==========================================
// Constant tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_constant() {
    // Windjammer constants should become Go const declarations
    let code = compile_to_go(
        r#"
const MAX_SIZE: int = 100

fn main() {
    println("ok")
}
"#,
    );
    assert!(
        code.contains("const MAX_SIZE"),
        "Should generate Go const. Got:\n{}",
        code
    );
}

// ==========================================
// Method receiver tests (expanded)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_method_with_mutation() {
    // Methods that mutate should use pointer receiver
    let output = compile_and_run_go(
        r#"
struct Counter {
    value: int
}

impl Counter {
    fn increment(self) {
        self.value += 1
    }

    fn get(self) -> int {
        self.value
    }
}

fn main() {
    let mut c = Counter { value: 0 }
    c.increment()
    c.increment()
    println("{}", c.get())
}
"#,
    );
    assert_eq!(output.trim(), "2");
}

// ==========================================
// Loop with break tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_loop_break() {
    // Loop with break should generate infinite for loop with break
    let output = compile_and_run_go(
        r#"
fn main() {
    let mut i = 0
    loop {
        if i >= 3 {
            break
        }
        println("{}", i)
        i += 1
    }
}
"#,
    );
    assert_eq!(output.trim(), "0\n1\n2");
}

// ==========================================
// Coverage gap: Continue statement (runtime)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_continue() {
    let output = compile_and_run_go(
        r#"
fn main() {
    let mut i = 0
    while i < 6 {
        i += 1
        if i % 2 == 0 {
            continue
        }
        println("{}", i)
    }
}
"#,
    );
    assert_eq!(output.trim(), "1\n3\n5");
}

// ==========================================
// Coverage gap: Recursion (runtime)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_recursion() {
    let output = compile_and_run_go(
        r#"
fn fibonacci(n: int) -> int {
    if n <= 1 {
        return n
    }
    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    println("{}", fibonacci(10))
}
"#,
    );
    assert_eq!(output.trim(), "55");
}

// ==========================================
// Coverage gap: For-range sum (runtime)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_go_for_range_sum() {
    let output = compile_and_run_go(
        r#"
fn main() {
    let mut sum = 0
    for i in 0..5 {
        sum += i
    }
    println("{}", sum)
}
"#,
    );
    assert_eq!(output.trim(), "10");
}
