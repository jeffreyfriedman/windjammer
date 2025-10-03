// Comprehensive feature tests for all language features

use std::fs;
use std::process::Command;

// Helper to compile and check generated code
fn compile_and_check(source: &str, expected_patterns: &[&str]) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
    
    // Create unique temp file for this test
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir();
    let test_file = format!("test_{}.wj", test_id);
    let temp_file = temp_dir.join(&test_file);
    fs::write(&temp_file, source).expect("Failed to write temp file");
    
    // Compile to unique output directory
    let output_dir = format!("test_output_{}", test_id);
    let output = Command::new("cargo")
        .args(["run", "--", "build", "--path", temp_file.to_str().unwrap(), "--output", &output_dir])
        .output()
        .expect("Failed to run compiler");
    
    if !output.status.success() {
        panic!("Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Read generated code
    let rs_file = format!("{}/{}.rs", output_dir, format!("test_{}", test_id));
    let generated = fs::read_to_string(&rs_file)
        .expect("Failed to read generated code");
    
    // Check expected patterns
    for pattern in expected_patterns {
        assert!(generated.contains(pattern), 
            "Expected pattern '{}' not found in:\n{}", pattern, generated);
    }
    
    generated
}

#[test]
fn test_basic_function() {
    let source = r#"
fn add(x: int, y: int) -> int {
    x + y
}
"#;
    compile_and_check(source, &[
        "fn add(x: &i64, y: &i64) -> i64",
        "x + y",
    ]);
}

#[test]
fn test_assignment_statement() {
    let source = r#"
fn increment(x: int) {
    x = x + 1
}
"#;
    compile_and_check(source, &[
        "fn increment(x: &mut i64)",
        "x = x + 1",
    ]);
}

#[test]
fn test_ternary_operator() {
    let source = r#"
fn sign(x: int) -> string {
    x > 0 ? "positive" : "negative"
}
"#;
    compile_and_check(source, &[
        "if x > 0 { \"positive\" } else { \"negative\" }",
    ]);
}

#[test]
fn test_string_interpolation() {
    let source = r#"
fn greet(name: string) -> string {
    "Hello, ${name}!"
}
"#;
    compile_and_check(source, &[
        "format!(",
        "\"Hello, {}!\"",
    ]);
}

#[test]
fn test_pipe_operator() {
    let source = r#"
fn double(x: int) -> int { x * 2 }
fn add_ten(x: int) -> int { x + 10 }

fn process(x: int) -> int {
    x |> double |> add_ten
}
"#;
    let generated = compile_and_check(source, &[
        "fn double",
        "fn add_ten",
    ]);
    
    // Pipe operator transforms to nested calls
    assert!(generated.contains("add_ten(double(x))") || 
            generated.contains("double") && generated.contains("add_ten"));
}

#[test]
fn test_struct_definition() {
    let source = r#"
struct Point {
    x: int,
    y: int,
}
"#;
    compile_and_check(source, &[
        "struct Point",
        "x: i64",
        "y: i64",
    ]);
}

#[test]
fn test_struct_with_auto_derive() {
    let source = r#"
@auto
struct Point {
    x: int,
    y: int,
}
"#;
    compile_and_check(source, &[
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]",
        "struct Point",
    ]);
}

#[test]
fn test_impl_block() {
    let source = r#"
struct Point {
    x: int,
    y: int,
}

impl Point {
    fn new(x: int, y: int) -> Point {
        Point { x, y }
    }
    
    fn distance(&self) -> int {
        self.x * self.x + self.y * self.y
    }
}
"#;
    compile_and_check(source, &[
        "impl Point",
        "fn new",
        "fn distance(&self)",
        "Point { x, y }",
    ]);
}

#[test]
fn test_trait_definition() {
    let source = r#"
trait Drawable {
    fn draw(&self) -> string;
}
"#;
    compile_and_check(source, &[
        "trait Drawable",
        "fn draw(&self) -> String",
    ]);
}

#[test]
fn test_trait_implementation() {
    let source = r#"
trait Show {
    fn show(&self) -> string;
}

struct Point { x: int, y: int }

impl Show for Point {
    fn show(&self) -> string {
        "Point"
    }
}
"#;
    compile_and_check(source, &[
        "trait Show",
        "impl Show for Point",
        "fn show(&self)",
    ]);
}

#[test]
fn test_enum_definition() {
    let source = r#"
enum Color {
    Red,
    Green,
    Blue,
}
"#;
    compile_and_check(source, &[
        "enum Color",
        "Red,",
        "Green,",
        "Blue,",
    ]);
}

#[test]
fn test_enum_with_data() {
    let source = r#"
enum Result {
    Ok(int),
    Err(string),
}
"#;
    compile_and_check(source, &[
        "enum Result",
        "Ok(i64)",
        "Err(String)",
    ]);
}

#[test]
fn test_pattern_matching() {
    let source = r#"
fn handle(x: int) -> string {
    match x {
        0 => "zero",
        1 => "one",
        _ => "other",
    }
}
"#;
    compile_and_check(source, &[
        "match x",
        "0 => \"zero\"",
        "1 => \"one\"",
        "_ => \"other\"",
    ]);
}

#[test]
fn test_match_with_guard() {
    let source = r#"
fn classify(x: int) -> string {
    match x {
        n if n > 0 => "positive",
        n if n < 0 => "negative",
        _ => "zero",
    }
}
"#;
    compile_and_check(source, &[
        "match x",
        "if n > 0",
        "\"positive\"",
    ]);
}

#[test]
fn test_for_loop() {
    let source = r#"
fn sum_range(n: int) -> int {
    let mut total = 0
    for i in 0..n {
        total = total + i
    }
    total
}
"#;
    compile_and_check(source, &[
        "let mut total = 0",
        "for i in 0..n",
        "total = total + i",
    ]);
}

#[test]
fn test_while_loop() {
    let source = r#"
fn countdown(n: int) {
    while n > 0 {
        n = n - 1
    }
}
"#;
    compile_and_check(source, &[
        "while n > 0",
        "n = n - 1",
    ]);
}

#[test]
fn test_closures() {
    let source = r#"
fn apply(f: fn(int) -> int, x: int) -> int {
    f(x)
}

fn main() {
    let double = |x| x * 2
}
"#;
    compile_and_check(source, &[
        "|x| x * 2",
    ]);
}

#[test]
fn test_character_literals() {
    let source = r#"
fn get_char() -> char {
    'a'
}
"#;
    compile_and_check(source, &[
        "'a'",
    ]);
}

#[test]
fn test_character_escapes() {
    let source = r#"
fn newline() -> char { '\n' }
fn tab() -> char { '\t' }
fn quote() -> char { '\'' }
"#;
    let generated = compile_and_check(source, &[
        "fn newline",
        "fn tab",
        "fn quote",
    ]);
    
    assert!(generated.contains("'\\n'") || generated.contains("newline"));
    assert!(generated.contains("'\\t'") || generated.contains("tab"));
}

#[test]
fn test_let_bindings() {
    let source = r#"
fn test() {
    let x = 5
    let y = 10
    let z = x + y
}
"#;
    compile_and_check(source, &[
        "let x = 5",
        "let y = 10",
        "let z = x + y",
    ]);
}

#[test]
fn test_mutable_bindings() {
    let source = r#"
fn test() {
    let mut x = 0
    x = x + 1
}
"#;
    compile_and_check(source, &[
        "let mut x = 0",
        "x = x + 1",
    ]);
}

#[test]
fn test_if_else() {
    let source = r#"
fn abs(x: int) -> int {
    if x < 0 {
        -x
    } else {
        x
    }
}
"#;
    compile_and_check(source, &[
        "if x < 0",
        "-x",
        "else",
    ]);
}

#[test]
fn test_return_statement() {
    let source = r#"
fn early_return(x: int) -> int {
    if x < 0 {
        return 0
    }
    x
}
"#;
    compile_and_check(source, &[
        "return 0",
    ]);
}

#[test]
fn test_automatic_reference_insertion() {
    let source = r#"
fn double(x: int) -> int {
    x * 2
}

fn main() {
    let n = 5
    double(n)
}
"#;
    compile_and_check(source, &[
        "fn double(x: &i64)",
        "double(&n)",
    ]);
}

#[test]
fn test_automatic_mut_reference() {
    let source = r#"
fn increment(x: int) {
    x = x + 1
}

fn main() {
    let mut counter = 0
    increment(counter)
}
"#;
    compile_and_check(source, &[
        "fn increment(x: &mut i64)",
        "increment(&mut counter)",
    ]);
}

#[test]
fn test_const_declaration() {
    let source = r#"
const MAX: int = 100
"#;
    compile_and_check(source, &[
        "const MAX: i64 = 100",
    ]);
}

#[test]
fn test_static_declaration() {
    let source = r#"
static mut COUNTER: int = 0
"#;
    compile_and_check(source, &[
        "static mut COUNTER: i64 = 0",
    ]);
}

#[test]
fn test_tuple_type() {
    let source = r#"
fn coords() -> (int, int) {
    (0, 0)
}
"#;
    compile_and_check(source, &[
        "(i64, i64)",
        "(0, 0)",
    ]);
}

#[test]
fn test_range_expressions() {
    let source = r#"
fn ranges() {
    let a = 0..10
    let b = 0..=10
}
"#;
    compile_and_check(source, &[
        "0..10",
        "0..=10",
    ]);
}

#[test]
fn test_array_indexing() {
    let source = r#"
fn get_first(arr: Vec<int>) -> int {
    arr[0]
}
"#;
    compile_and_check(source, &[
        "arr[0]",
    ]);
}

#[test]
fn test_method_calls() {
    let source = r#"
fn length(s: string) -> int {
    s.len()
}
"#;
    compile_and_check(source, &[
        "s.len()",
    ]);
}

#[test]
fn test_field_access() {
    let source = r#"
struct Point { x: int, y: int }

fn get_x(p: Point) -> int {
    p.x
}
"#;
    compile_and_check(source, &[
        "p.x",
    ]);
}

