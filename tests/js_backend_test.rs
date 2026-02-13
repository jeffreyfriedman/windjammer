/// Integration tests for the JavaScript backend (TDD)
///
/// Tests impl block → class method generation, match expressions,
/// and ensures the JS backend produces valid Node.js-executable code.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Compile .wj source to JavaScript and return the generated JS code
fn compile_to_js(source: &str) -> String {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("javascript")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "JS compilation failed:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    // Find the generated JS file (the backend names it output.js)
    for name in &["output.js", "main.js", "test.js"] {
        let generated_file = output_dir.join(name);
        if generated_file.exists() {
            return fs::read_to_string(&generated_file).unwrap();
        }
    }

    // List what's in the output directory
    let entries: Vec<String> = fs::read_dir(&output_dir)
        .map(|dir| {
            dir.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    panic!(
        "No JS file found in output dir. Files: {:?}\nStderr:\n{}",
        entries,
        String::from_utf8_lossy(&wj_output.stderr)
    );
}

/// Compile .wj to JS and run with Node.js. Returns stdout.
fn compile_and_run_js(source: &str) -> String {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("javascript")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "JS codegen failed:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    // Find JS file (the backend names it output.js)
    let js_file = ["output.js", "main.js", "test.js"]
        .iter()
        .map(|n| output_dir.join(n))
        .find(|p| p.exists())
        .unwrap_or_else(|| {
            let entries: Vec<String> = fs::read_dir(&output_dir)
                .map(|dir| {
                    dir.filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect()
                })
                .unwrap_or_default();
            panic!(
                "No JS file found. Files: {:?}\nStderr:\n{}",
                entries,
                String::from_utf8_lossy(&wj_output.stderr)
            );
        });

    // The output directory has a package.json with "type": "module"
    // but `import.meta.url` doesn't match when running via `node output.js`,
    // so we need to append a direct main() call for testing.
    let js_code = fs::read_to_string(&js_file).unwrap();
    // Write a .mjs file with the code + unconditional main() call
    let test_js = output_dir.join("_test.mjs");
    let test_code = format!(
        "{}\n\n// Test runner: unconditional main() call\nif (typeof main === 'function') main();\n",
        // Remove `export` keyword so it works as a standalone script
        js_code.replace("export function", "function").replace("export class", "class").replace("export const", "const").replace("export let", "let")
    );
    fs::write(&test_js, &test_code).unwrap();

    let node_output = Command::new("node")
        .arg(&test_js)
        .current_dir(&output_dir)
        .output()
        .expect("Failed to execute node");

    if !node_output.status.success() {
        let generated = fs::read_to_string(&js_file).unwrap_or_default();
        panic!(
            "Node.js execution failed:\n{}\n\nGenerated code:\n{}",
            String::from_utf8_lossy(&node_output.stderr),
            generated
        );
    }

    String::from_utf8(node_output.stdout).unwrap()
}

// ==========================================
// Basic JS generation tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_hello_world() {
    let output = compile_and_run_js(
        r#"
fn main() {
    println("Hello from JS!")
}
"#,
    );
    assert_eq!(output.trim(), "Hello from JS!");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_arithmetic() {
    let output = compile_and_run_js(
        r#"
fn main() {
    let a = 2 + 3
    println("{}", a)
}
"#,
    );
    assert_eq!(output.trim(), "5");
}

// ==========================================
// Impl block → class method tests (THE KEY FIX)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_impl_generates_methods() {
    // Impl blocks should add methods to the corresponding class.
    // Currently they are dropped entirely — this test should FAIL (RED phase).
    let code = compile_to_js(
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
    println("ok")
}
"#,
    );
    assert!(
        code.contains("sum(") || code.contains("sum ("),
        "Impl method 'sum' should appear in generated class. Got:\n{}",
        code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_impl_methods_callable() {
    // Methods should be callable on instances
    let output = compile_and_run_js(
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
    );
    assert_eq!(output.trim(), "7");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_impl_multiple_methods() {
    // Multiple methods in an impl block should all be generated
    let code = compile_to_js(
        r#"
struct Rect {
    w: float,
    h: float
}

impl Rect {
    fn area(self) -> float {
        self.w * self.h
    }

    fn perimeter(self) -> float {
        2.0 * (self.w + self.h)
    }
}

fn main() {
    println("ok")
}
"#,
    );
    assert!(
        code.contains("area(") || code.contains("area ("),
        "Should contain area method. Got:\n{}",
        code
    );
    assert!(
        code.contains("perimeter(") || code.contains("perimeter ("),
        "Should contain perimeter method. Got:\n{}",
        code
    );
}

// ==========================================
// Match expression tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_match_as_expression() {
    // Match used as an expression should work in JS
    let output = compile_and_run_js(
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
    println("{}", describe(3))
}
"#,
    );
    assert_eq!(output.trim(), "one\nother");
}

// ==========================================
// Struct with constructor tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_struct_generates_class() {
    let code = compile_to_js(
        r#"
struct Player {
    name: string,
    score: int
}

fn main() {
    println("ok")
}
"#,
    );
    assert!(
        code.contains("class Player"),
        "Struct should generate class. Got:\n{}",
        code
    );
    assert!(
        code.contains("constructor("),
        "Class should have constructor. Got:\n{}",
        code
    );
}

// ==========================================
// Control flow tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_if_else() {
    let output = compile_and_run_js(
        r#"
fn main() {
    let x = 10
    if x > 5 {
        println("big")
    } else {
        println("small")
    }
}
"#,
    );
    assert_eq!(output.trim(), "big");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_while_loop() {
    let output = compile_and_run_js(
        r#"
fn main() {
    let mut i = 0
    while i < 3 {
        println("{}", i)
        i += 1
    }
}
"#,
    );
    assert_eq!(output.trim(), "0\n1\n2");
}

// ==========================================
// Function tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_function_with_return() {
    let output = compile_and_run_js(
        r#"
fn double(n: int) -> int {
    n * 2
}

fn main() {
    println("{}", double(21))
}
"#,
    );
    assert_eq!(output.trim(), "42");
}

// ==========================================
// Enum tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_enum_generation() {
    // Enums should generate Object.freeze or similar construct
    let code = compile_to_js(
        r#"
enum Direction {
    Up,
    Down,
    Left,
    Right
}

fn main() {
    println("ok")
}
"#,
    );
    assert!(
        code.contains("Direction") && (code.contains("Object.freeze") || code.contains("Symbol")),
        "Enum should generate JS enum pattern. Got:\n{}",
        code
    );
}

// ==========================================
// Coverage gap: Recursion (runtime)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_recursion() {
    let output = compile_and_run_js(
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
// Coverage gap: Struct mutation (runtime)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_struct_mutation() {
    let output = compile_and_run_js(
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
}
"#,
    );
    assert_eq!(output.trim(), "0\n1\n2");
}

// ==========================================
// Coverage gap: For-range (runtime)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_for_range() {
    let output = compile_and_run_js(
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

// ==========================================
// Coverage gap: Continue statement (runtime)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_continue() {
    let output = compile_and_run_js(
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
// Coverage gap: Loop/break (runtime)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_loop_break() {
    let output = compile_and_run_js(
        r#"
fn main() {
    let mut count = 0
    loop {
        if count >= 3 {
            break
        }
        println("{}", count)
        count += 1
    }
}
"#,
    );
    assert_eq!(output.trim(), "0\n1\n2");
}
