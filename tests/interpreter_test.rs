/// Windjammerscript Interpreter Tests (TDD)
///
/// Proves the interpreter produces identical output to compiled backends.
/// Same .wj source → interpreted output == compiled output.
/// This is the key guarantee: quick iteration with interpreter,
/// then export to Rust for production.

// We use the parser directly + interpreter, without going through the CLI.
// This tests the interpreter engine in isolation.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Parse source and run through the interpreter, capturing output
fn interpret(source: &str) -> Result<String, String> {
    // Lex
    let mut lexer = windjammer::lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();

    // Parse
    let mut parser = windjammer::parser::Parser::new_with_source(
        tokens,
        "test.wj".to_string(),
        source.to_string(),
    );
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    // Run interpreter
    let mut interp = windjammer::interpreter::Interpreter::new_capturing();
    interp.run(&program)?;
    Ok(interp.get_output())
}

/// Compile to Rust and run, returning stdout
fn compile_and_run_rust(source: &str) -> String {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let wj = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj");

    assert!(wj.status.success(), "wj failed: {}", String::from_utf8_lossy(&wj.stderr));

    let rs_file = output_dir.join("test.rs");
    let bin_path = temp_dir.path().join("test_bin");
    let rustc = Command::new("rustc")
        .arg(&rs_file)
        .arg("-o")
        .arg(&bin_path)
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to execute rustc");

    assert!(rustc.status.success(), "rustc failed: {}", String::from_utf8_lossy(&rustc.stderr));

    let run = Command::new(&bin_path).output().expect("Failed to run");
    String::from_utf8(run.stdout).unwrap()
}

/// Assert interpreter output matches compiled Rust output
fn assert_interpreter_matches_compiled(test_name: &str, source: &str) {
    let interp_output = interpret(source).unwrap_or_else(|e| panic!("[{}] Interpreter error: {}", test_name, e));
    let compiled_output = compile_and_run_rust(source);

    assert_eq!(
        interp_output, compiled_output,
        "[{}] Interpreter vs Compiled mismatch!\nInterpreter:\n{}\nCompiled:\n{}",
        test_name, interp_output, compiled_output
    );
}

// ==========================================
// Conformance: interpreter == compiled
// ==========================================

#[test]
fn test_interpret_hello_world() {
    assert_interpreter_matches_compiled(
        "hello_world",
        r#"
fn main() {
    println("Hello, world!")
}
"#,
    );
}

#[test]
fn test_interpret_arithmetic() {
    assert_interpreter_matches_compiled(
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
    );
}

#[test]
fn test_interpret_control_flow() {
    assert_interpreter_matches_compiled(
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
    );
}

#[test]
fn test_interpret_functions() {
    assert_interpreter_matches_compiled(
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
    );
}

#[test]
fn test_interpret_recursion() {
    assert_interpreter_matches_compiled(
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
    );
}

#[test]
fn test_interpret_structs() {
    assert_interpreter_matches_compiled(
        "structs",
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
    );
}

#[test]
fn test_interpret_mutation() {
    assert_interpreter_matches_compiled(
        "mutation",
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
    );
}

#[test]
fn test_interpret_loop_break() {
    assert_interpreter_matches_compiled(
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
    );
}

#[test]
fn test_interpret_match() {
    assert_interpreter_matches_compiled(
        "match",
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
    );
}

#[test]
fn test_interpret_nested_if() {
    assert_interpreter_matches_compiled(
        "nested_if",
        r#"
fn classify(n: int) -> string {
    if n > 0 {
        if n > 100 {
            "big positive"
        } else {
            "small positive"
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
    );
}

#[test]
fn test_interpret_variable_shadowing() {
    assert_interpreter_matches_compiled(
        "shadowing",
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
    );
}

// ==========================================
// Interpreter-only tests (fast, no compilation)
// ==========================================

#[test]
fn test_interpret_boolean_logic() {
    let output = interpret(
        r#"
fn main() {
    let a = true
    let b = false
    if a && !b {
        println("correct")
    }
    if a || b {
        println("or works")
    }
}
"#,
    ).unwrap();
    assert!(output.contains("correct"));
    assert!(output.contains("or works"));
}

#[test]
fn test_interpret_string_formatting() {
    let output = interpret(
        r#"
fn main() {
    let name = "World"
    println("Hello, {}!", name)
}
"#,
    ).unwrap();
    assert_eq!(output.trim(), "Hello, World!");
}

#[test]
fn test_interpret_for_range() {
    let output = interpret(
        r#"
fn main() {
    let mut sum = 0
    for i in 0..5 {
        sum += i
    }
    println("{}", sum)
}
"#,
    ).unwrap();
    assert_eq!(output.trim(), "10");
}
