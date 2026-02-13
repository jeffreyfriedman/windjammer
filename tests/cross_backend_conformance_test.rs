/// Cross-Backend Conformance Tests
///
/// Verifies that the same .wj source produces identical stdout output
/// when compiled to Rust, Go, JavaScript, AND interpreted by Windjammerscript.
///
/// This is the ultimate proof that all execution modes are semantically equivalent:
/// - `wj build --target rust` (compiled)
/// - `wj build --target go` (compiled)
/// - `wj build --target javascript` (compiled)
/// - `wj run --interpret` (interpreted)
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Result of compiling and running a .wj file on a specific backend
#[allow(dead_code)]
struct BackendResult {
    backend: String,
    stdout: String,
    success: bool,
    error: String,
}

fn compile_and_run_rust(source: &str) -> BackendResult {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    // Compile .wj → Rust (wj writes to ./build/ relative to cwd)
    let wj = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj");

    if !wj.status.success() {
        return BackendResult {
            backend: "rust".into(),
            stdout: String::new(),
            success: false,
            error: format!(
                "wj compilation failed: {}",
                String::from_utf8_lossy(&wj.stderr)
            ),
        };
    }

    // Find generated .rs file (named after input: test.wj → test.rs)
    let rs_file = output_dir.join("test.rs");
    if !rs_file.exists() {
        // Try finding any .rs file
        let entries: Vec<String> = fs::read_dir(&output_dir)
            .map(|dir| {
                dir.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        return BackendResult {
            backend: "rust".into(),
            stdout: String::new(),
            success: false,
            error: format!("No test.rs found. Files in build/: {:?}", entries),
        };
    }

    // Compile with rustc
    let bin_path = temp_dir.path().join("test_bin");
    let rustc = Command::new("rustc")
        .arg(&rs_file)
        .arg("-o")
        .arg(&bin_path)
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to execute rustc");

    if !rustc.status.success() {
        let rs_code = fs::read_to_string(&rs_file).unwrap_or_default();
        return BackendResult {
            backend: "rust".into(),
            stdout: String::new(),
            success: false,
            error: format!(
                "rustc failed: {}\n\nGenerated Rust:\n{}",
                String::from_utf8_lossy(&rustc.stderr),
                rs_code
            ),
        };
    }

    // Run the binary
    let run = Command::new(&bin_path)
        .output()
        .expect("Failed to run binary");

    BackendResult {
        backend: "rust".into(),
        stdout: String::from_utf8(run.stdout).unwrap_or_default(),
        success: run.status.success(),
        error: String::from_utf8(run.stderr).unwrap_or_default(),
    }
}

fn compile_and_run_go(source: &str) -> BackendResult {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let wj = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("go")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj");

    if !wj.status.success() {
        return BackendResult {
            backend: "go".into(),
            stdout: String::new(),
            success: false,
            error: format!(
                "wj compilation failed: {}",
                String::from_utf8_lossy(&wj.stderr)
            ),
        };
    }

    let go_file = output_dir.join("main.go");
    let run = Command::new("go")
        .arg("run")
        .arg(&go_file)
        .current_dir(&output_dir)
        .output()
        .expect("Failed to execute go run");

    BackendResult {
        backend: "go".into(),
        stdout: String::from_utf8(run.stdout).unwrap_or_default(),
        success: run.status.success(),
        error: String::from_utf8(run.stderr).unwrap_or_default(),
    }
}

fn compile_and_run_js(source: &str) -> BackendResult {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let wj = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("javascript")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj");

    if !wj.status.success() {
        return BackendResult {
            backend: "js".into(),
            stdout: String::new(),
            success: false,
            error: format!(
                "wj compilation failed: {}",
                String::from_utf8_lossy(&wj.stderr)
            ),
        };
    }

    // Find JS file and create a runnable version
    let js_file = output_dir.join("output.js");
    if !js_file.exists() {
        return BackendResult {
            backend: "js".into(),
            stdout: String::new(),
            success: false,
            error: "No output.js generated".into(),
        };
    }

    let js_code = fs::read_to_string(&js_file).unwrap();
    let runner = output_dir.join("_run.mjs");
    let runner_code = format!(
        "{}\nif (typeof main === 'function') main();\n",
        js_code
            .replace("export function", "function")
            .replace("export class", "class")
            .replace("export const", "const")
            .replace("export let", "let")
    );
    fs::write(&runner, &runner_code).unwrap();

    let run = Command::new("node")
        .arg(&runner)
        .current_dir(&output_dir)
        .output()
        .expect("Failed to execute node");

    BackendResult {
        backend: "js".into(),
        stdout: String::from_utf8(run.stdout).unwrap_or_default(),
        success: run.status.success(),
        error: String::from_utf8(run.stderr).unwrap_or_default(),
    }
}

/// Run source through the Windjammerscript interpreter, capturing output
fn interpret(source: &str) -> BackendResult {
    let mut lex = windjammer::lexer::Lexer::new(source);
    let tokens = lex.tokenize_with_locations();
    let mut parse = windjammer::parser::Parser::new_with_source(
        tokens,
        "conformance_test.wj".to_string(),
        source.to_string(),
    );
    let program = match parse.parse() {
        Ok(p) => p,
        Err(e) => {
            return BackendResult {
                backend: "interpreter".into(),
                stdout: String::new(),
                success: false,
                error: format!("Parse error: {}", e),
            };
        }
    };

    let mut interp = windjammer::interpreter::Interpreter::new_capturing();
    match interp.run(&program) {
        Ok(_) => BackendResult {
            backend: "interpreter".into(),
            stdout: interp.get_output(),
            success: true,
            error: String::new(),
        },
        Err(e) => BackendResult {
            backend: "interpreter".into(),
            stdout: interp.get_output(),
            success: false,
            error: e,
        },
    }
}

/// Assert Rust + Interpreter produce identical output (for features Go/JS don't support yet)
fn assert_rust_and_interpreter_agree(test_name: &str, source: &str, expected_contains: &str) {
    let rust_result = compile_and_run_rust(source);
    let interp_result = interpret(source);

    assert!(
        rust_result.success,
        "[{}] Rust backend failed: {}",
        test_name, rust_result.error
    );
    assert!(
        interp_result.success,
        "[{}] Interpreter failed: {}",
        test_name, interp_result.error
    );

    assert!(
        rust_result.stdout.contains(expected_contains),
        "[{}] Rust output missing '{}'. Got:\n{}",
        test_name,
        expected_contains,
        rust_result.stdout
    );

    assert_eq!(
        rust_result.stdout, interp_result.stdout,
        "[{}] Rust vs Interpreter output mismatch!\nRust:\n{}\nInterpreter:\n{}",
        test_name, rust_result.stdout, interp_result.stdout
    );
}

/// Assert ALL backends (Rust, Go, JS, Interpreter) produce identical output
fn assert_backends_agree(test_name: &str, source: &str, expected_contains: &str) {
    let rust_result = compile_and_run_rust(source);
    let go_result = compile_and_run_go(source);
    let js_result = compile_and_run_js(source);
    let interp_result = interpret(source);

    // All must succeed
    assert!(
        rust_result.success,
        "[{}] Rust backend failed: {}",
        test_name, rust_result.error
    );
    assert!(
        go_result.success,
        "[{}] Go backend failed: {}",
        test_name, go_result.error
    );
    assert!(
        js_result.success,
        "[{}] JS backend failed: {}",
        test_name, js_result.error
    );
    assert!(
        interp_result.success,
        "[{}] Interpreter failed: {}",
        test_name, interp_result.error
    );

    // All must contain expected output
    assert!(
        rust_result.stdout.contains(expected_contains),
        "[{}] Rust output missing '{}'. Got:\n{}",
        test_name,
        expected_contains,
        rust_result.stdout
    );

    // All four must produce identical output
    assert_eq!(
        rust_result.stdout, go_result.stdout,
        "[{}] Rust vs Go output mismatch!\nRust:\n{}\nGo:\n{}",
        test_name, rust_result.stdout, go_result.stdout
    );

    assert_eq!(
        rust_result.stdout, js_result.stdout,
        "[{}] Rust vs JS output mismatch!\nRust:\n{}\nJS:\n{}",
        test_name, rust_result.stdout, js_result.stdout
    );

    assert_eq!(
        rust_result.stdout, interp_result.stdout,
        "[{}] Rust vs Interpreter output mismatch!\nRust:\n{}\nInterpreter:\n{}",
        test_name, rust_result.stdout, interp_result.stdout
    );
}

// ==========================================
// Cross-backend conformance tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_hello_world() {
    assert_backends_agree(
        "hello_world",
        r#"
fn main() {
    println("Hello, world!")
}
"#,
        "Hello, world!",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_arithmetic() {
    assert_backends_agree(
        "arithmetic",
        r#"
fn main() {
    let a = 1 + 2
    println("[add] {}", a)
    let b = 10 - 3
    println("[sub] {}", b)
    let c = 6 * 7
    println("[mul] {}", c)
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_control_flow() {
    assert_backends_agree(
        "control_flow",
        r#"
fn main() {
    let a = 5
    if a > 0 {
        println("[if] positive")
    } else {
        println("[if] non-positive")
    }

    let mut i = 0
    while i < 3 {
        println("[while] {}", i)
        i += 1
    }

    for j in 0..3 {
        println("[for] {}", j)
    }

    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_functions() {
    assert_backends_agree(
        "functions",
        r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(10, 20)
    println("[add] {}", result)
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_structs_and_methods() {
    assert_backends_agree(
        "structs_and_methods",
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
    println("[sum] {}", p.sum())
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_loop_break() {
    assert_backends_agree(
        "loop_break",
        r#"
fn main() {
    let mut count = 0
    loop {
        if count >= 3 {
            break
        }
        println("[loop] {}", count)
        count += 1
    }
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_match_values() {
    assert_backends_agree(
        "match_values",
        r#"
fn describe(x: int) -> string {
    match x {
        1 => "one",
        2 => "two",
        _ => "other"
    }
}

fn main() {
    println("[match] {}", describe(1))
    println("[match] {}", describe(2))
    println("[match] {}", describe(99))
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_recursion() {
    assert_backends_agree(
        "recursion",
        r#"
fn fibonacci(n: int) -> int {
    if n <= 1 {
        return n
    }
    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    println("[fib] {}", fibonacci(10))
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_nested_if_expression() {
    // Rust + Interpreter only: Go treats if/else as statements, not expressions.
    // Promoting to all backends requires Go codegen to emit helper variables or
    // closures for if/else expressions that return values outside of match context.
    assert_rust_and_interpreter_agree(
        "nested_if",
        r#"
fn classify(n: int) -> string {
    if n > 0 {
        if n > 100 {
            "big"
        } else {
            "small"
        }
    } else if n == 0 {
        "zero"
    } else {
        "negative"
    }
}

fn main() {
    println("{}", classify(500))
    println("{}", classify(5))
    println("{}", classify(0))
    println("{}", classify(-3))
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// test_conformance_enum_basic removed: superseded by test_conformance_enum_unit_all_backends
// which verifies all 4 backends (Rust, Go, JS, Interpreter) produce identical output.

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_enum_with_data() {
    // Rust + Interpreter only: Go/JS need constructor functions for data-carrying
    // enum variants (e.g., Shape::Circle(5)) and destructuring in match arms.
    // Unit enum variants (Color::Red) work on all backends; data variants do not yet.
    assert_rust_and_interpreter_agree(
        "enum_data",
        r#"
enum Shape {
    Circle(int),
    Square(int),
    Point,
}

fn area(s: Shape) -> int {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Square(side) => side * side,
        Shape::Point => 0,
    }
}

fn main() {
    println("{}", area(Shape::Circle(5)))
    println("{}", area(Shape::Square(4)))
    println("{}", area(Shape::Point))
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_struct_mutation() {
    assert_backends_agree(
        "struct_mutation",
        r#"
struct Counter {
    value: int
}

impl Counter {
    fn get(self) -> int {
        self.value
    }

    fn increment(self) {
        self.value += 1
    }
}

fn main() {
    let mut c = Counter { value: 0 }
    println("{}", c.get())
    c.increment()
    println("{}", c.get())
    c.increment()
    println("{}", c.get())
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_for_sum() {
    assert_backends_agree(
        "for_sum",
        r#"
fn main() {
    let mut sum = 0
    for i in 0..10 {
        sum += i
    }
    println("{}", sum)
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// Coverage gap: Continue statement (all 4 backends)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_continue_while() {
    assert_backends_agree(
        "continue_while",
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
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_continue_for() {
    assert_backends_agree(
        "continue_for",
        r#"
fn main() {
    for i in 0..8 {
        if i % 3 == 0 {
            continue
        }
        println("{}", i)
    }
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// Coverage gap: Multiple functions calling each other
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_function_composition() {
    assert_backends_agree(
        "function_composition",
        r#"
fn double(x: int) -> int {
    x * 2
}

fn add_one(x: int) -> int {
    x + 1
}

fn main() {
    let a = double(add_one(3))
    let b = add_one(double(3))
    println("{}", a)
    println("{}", b)
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// Coverage gap: Boolean logic and comparisons
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_boolean_logic() {
    assert_backends_agree(
        "boolean_logic",
        r#"
fn main() {
    let t = true
    let f = false
    if t && !f {
        println("and_not")
    }
    if t || f {
        println("or")
    }
    if 5 >= 3 && 3 <= 5 {
        println("compare")
    }
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// Coverage gap: Nested loops
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_nested_loops() {
    assert_backends_agree(
        "nested_loops",
        r#"
fn main() {
    for i in 0..3 {
        for j in 0..3 {
            if i == j {
                println("{}", i)
            }
        }
    }
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// test_conformance_match_guards removed: superseded by test_conformance_match_guards_all_backends
// which verifies all 4 backends produce identical output.

// test_conformance_variable_shadowing removed: superseded by test_conformance_shadowing_all_backends
// which verifies all 4 backends produce identical output (JS shadowing fixed via variable rename pass).

// ==========================================
// Coverage gap: Multiple struct methods
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_multi_method_struct() {
    assert_backends_agree(
        "multi_method_struct",
        r#"
struct Rect {
    w: int,
    h: int
}

impl Rect {
    fn area(self) -> int {
        self.w * self.h
    }

    fn perimeter(self) -> int {
        2 * (self.w + self.h)
    }
}

fn main() {
    let r = Rect { w: 5, h: 3 }
    println("{}", r.area())
    println("{}", r.perimeter())
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// NEW: Promote enum tests to all 4 backends
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_enum_unit_all_backends() {
    assert_backends_agree(
        "enum_unit_all",
        r#"
enum Color {
    Red,
    Green,
    Blue,
}

fn color_name(c: Color) -> string {
    match c {
        Color::Red => "red",
        Color::Green => "green",
        Color::Blue => "blue",
    }
}

fn main() {
    println("{}", color_name(Color::Red))
    println("{}", color_name(Color::Green))
    println("{}", color_name(Color::Blue))
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// NEW: Match guards — all 4 backends
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_match_guards_all_backends() {
    assert_backends_agree(
        "match_guards_all",
        r#"
fn classify(n: int) -> string {
    match n {
        x if x > 100 => "big",
        x if x > 0 => "small",
        0 => "zero",
        _ => "negative",
    }
}

fn main() {
    println("{}", classify(500))
    println("{}", classify(5))
    println("{}", classify(0))
    println("{}", classify(-3))
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// NEW: String interpolation — all 4 backends
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_string_interpolation() {
    assert_backends_agree(
        "string_interpolation",
        r#"
fn main() {
    let name = "world"
    let x = 42
    println("Hello, ${name}! The answer is ${x}.")
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// NEW: Variable shadowing — all 4 backends
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_shadowing_all_backends() {
    assert_backends_agree(
        "shadowing_all",
        r#"
fn main() {
    let x = 10
    println("{}", x)
    let x = 20
    println("{}", x)
    let x = x + 5
    println("{}", x)
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// NEW: Static constructor (Type::new) — all 4 backends
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_static_constructor() {
    assert_backends_agree(
        "static_constructor",
        r#"
struct Point {
    x: int,
    y: int
}

impl Point {
    fn new(x: int, y: int) -> Point {
        Point { x: x, y: y }
    }
}

fn main() {
    let p = Point::new(3, 4)
    println("{} {}", p.x, p.y)
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// NEW: Vec operations — all 4 backends
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_vec_push_len() {
    assert_backends_agree(
        "vec_push_len",
        r#"
fn main() {
    let mut v = vec![1, 2, 3]
    v.push(4)
    println("{}", v.len())
    println("{}", v[0])
    println("{}", v[3])
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// NEW: Nested function calls — all 4 backends
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_nested_function_calls() {
    assert_backends_agree(
        "nested_calls",
        r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn mul(a: int, b: int) -> int {
    a * b
}

fn main() {
    println("{}", add(mul(2, 3), mul(4, 5)))
    println("PASSED")
}
"#,
        "PASSED",
    );
}

// ==========================================
// NEW: Multiple return paths — all 4 backends
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_early_return() {
    assert_backends_agree(
        "early_return",
        r#"
fn abs(n: int) -> int {
    if n < 0 {
        return -n
    }
    n
}

fn main() {
    println("{}", abs(-5))
    println("{}", abs(3))
    println("{}", abs(0))
    println("PASSED")
}
"#,
        "PASSED",
    );
}
