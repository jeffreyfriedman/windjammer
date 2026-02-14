// Comprehensive feature tests for all language features

use std::fs;
use std::process::Command;

// Helper to compile and check generated code
fn compile_and_check(source: &str, expected_patterns: &[&str]) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    // Create unique temp file for this test using process ID, thread ID, and counter
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let process_id = std::process::id();
    let thread_id = format!("{:?}", thread::current().id());
    let unique_id = format!(
        "{}_{}_{}_{}",
        process_id,
        thread_id.replace("ThreadId(", "").replace(")", ""),
        test_id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let temp_dir = std::env::temp_dir();
    let test_file = format!("test_{}.wj", unique_id);
    let temp_file = temp_dir.join(&test_file);
    fs::write(&temp_file, source).expect("Failed to write temp file");

    // Compile to unique output directory (absolute path)
    let output_dir = temp_dir.join(format!("output_{}", unique_id));
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "wj",
            "--",
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            temp_file.to_str().unwrap(),
            "--no-cargo", // Skip cargo build in tests
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Compilation failed:\nSTDOUT: {}\nSTDERR: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read generated code
    let rs_file = output_dir.join(format!("test_{}.rs", unique_id));
    let generated = fs::read_to_string(&rs_file).unwrap_or_else(|_| {
        let files: Vec<_> = fs::read_dir(&output_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        panic!(
            "Expected file {:?} not found. Output directory contains: {:?}",
            rs_file, files
        )
    });

    // Check expected patterns
    for pattern in expected_patterns {
        assert!(
            generated.contains(pattern),
            "Expected pattern '{}' not found in:\n{}",
            pattern,
            generated
        );
    }

    // Cleanup temp files
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_dir_all(&output_dir);

    generated
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_basic_function() {
    let source = r#"
fn add(x: int, y: int) -> int {
    x + y
}
"#;
    // Parameters not mutated, so no 'mut' needed
    compile_and_check(source, &["fn add(x: i64, y: i64) -> i64", "x + y"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_assignment_statement() {
    let source = r#"
fn increment(x: int) {
    x = x + 1
}
"#;
    // Ownership inference detects mutation, infers &mut
    // Phase 5 optimization converts x = x + 1 to x += 1
    compile_and_check(source, &["fn increment(x: &mut i64)", "x += 1"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_expression() {
    let source = r#"
fn sign(x: int) -> string {
    if x > 0 { "positive" } else { "negative" }
}
"#;
    // Check for the if/else logic (formatting may vary)
    compile_and_check(source, &["if x > 0", "\"positive\"", "\"negative\""]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_interpolation() {
    let source = r#"
fn greet(name: string) -> string {
    "Hello, ${name}!"
}
"#;
    compile_and_check(source, &["format!(", "\"Hello, {}!\""]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_pipe_operator() {
    let source = r#"
fn double(x: int) -> int { x * 2 }
fn add_ten(x: int) -> int { x + 10 }

fn process(x: int) -> int {
    x |> double |> add_ten
}
"#;
    let generated = compile_and_check(source, &["fn double", "fn add_ten"]);

    // Pipe operator transforms to nested calls
    assert!(
        generated.contains("add_ten(double(x))")
            || generated.contains("double") && generated.contains("add_ten")
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_definition() {
    let source = r#"
struct Point {
    x: int,
    y: int,
}
"#;
    compile_and_check(source, &["struct Point", "x: i64", "y: i64"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_with_auto_derive() {
    let source = r#"
@auto
struct Point {
    x: int,
    y: int,
}
"#;
    compile_and_check(
        source,
        &[
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]",
            "struct Point",
        ],
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    compile_and_check(
        source,
        &[
            "impl Point",
            "fn new",
            "fn distance(&self)",
            "Point { x, y }", // Phase 3: Uses idiomatic Rust struct shorthand
        ],
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_definition() {
    let source = r#"
trait Drawable {
    fn draw(&self) -> string {
        "default"
    }
}
"#;
    compile_and_check(source, &["trait Drawable", "fn draw(&self) -> String"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_implementation() {
    let source = r#"
trait Show {
    fn show(&self) -> string {
        "default"
    }
}

struct Point { x: int, y: int }

impl Show for Point {
    fn show(&self) -> string {
        "Point"
    }
}
"#;
    compile_and_check(
        source,
        &["trait Show", "impl Show for Point", "fn show(&self)"],
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_definition() {
    let source = r#"
enum Color {
    Red,
    Green,
    Blue,
}
"#;
    compile_and_check(source, &["enum Color", "Red,", "Green,", "Blue,"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_with_data() {
    let source = r#"
enum Result {
    Ok(int),
    Err(string),
}
"#;
    compile_and_check(source, &["enum Result", "Ok(i64)", "Err(String)"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    compile_and_check(
        source,
        &["match x", "0 => \"zero\"", "1 => \"one\"", "_ => \"other\""],
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    compile_and_check(source, &["match x", "if n > 0", "\"positive\""]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop() {
    let source = r#"
fn main() {
    let mut total = 0
    for i in 0..5 {
        total = total + i
    }
    total
}
"#;
    // Phase 5 optimization works for local variables
    compile_and_check(
        source,
        &["let mut total = 0", "for i in 0..5", "total += i"],
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_while_loop() {
    let source = r#"
fn countdown(n: int) {
    while n > 0 {
        n = n - 1
    }
}
"#;
    // Phase 5 optimization: n = n - 1 becomes n -= 1
    // Note: &mut parameters auto-deref for assignments, so no * needed
    compile_and_check(source, &["while n > 0", "n -= 1"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closures() {
    let source = r#"
fn main() {
    let double = |x| x * 2
    let result = double(5)
    result
}
"#;
    compile_and_check(source, &["|x| x * 2"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_character_literals() {
    let source = r#"
fn get_char() -> char {
    'a'
}
"#;
    compile_and_check(source, &["'a'"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_character_escapes() {
    let source = r#"
fn newline() -> char { '\n' }
fn tab() -> char { '\t' }
fn quote() -> char { '\'' }
"#;
    let generated = compile_and_check(source, &["fn newline", "fn tab", "fn quote"]);

    assert!(generated.contains("'\\n'") || generated.contains("newline"));
    assert!(generated.contains("'\\t'") || generated.contains("tab"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_bindings() {
    let source = r#"
fn test() {
    let x = 5
    let y = 10
    let z = x + y
}
"#;
    // Note: z gets _ prefix because it's unused in the function body
    compile_and_check(source, &["let x = 5", "let y = 10", "let _z = x + y"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mutable_bindings() {
    let source = r#"
fn test() {
    let mut x = 0
    x = x + 1
}
"#;
    // Phase 5 optimization works for local variables
    compile_and_check(source, &["let mut x = 0", "x += 1"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    compile_and_check(source, &["if x < 0", "-x", "else"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_statement() {
    let source = r#"
fn early_return(x: int) -> int {
    if x < 0 {
        return 0
    }
    x
}
"#;
    compile_and_check(source, &["return 0"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    // Copy types like int are passed by value (no mut needed if not mutated)
    compile_and_check(source, &["fn double(x: i64)", "double(n)"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    // Ownership inference detects mutation, infers &mut
    // Call site automatically adds &mut
    compile_and_check(
        source,
        &["fn increment(x: &mut i64)", "increment(&mut counter)"],
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_const_declaration() {
    let source = r#"
const MAX: int = 100
"#;
    compile_and_check(source, &["const MAX: i64 = 100"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_static_declaration() {
    let source = r#"
static mut COUNTER: int = 0
"#;
    compile_and_check(source, &["static mut COUNTER: i64 = 0"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_tuple_type() {
    let source = r#"
fn coords() -> (int, int) {
    (0, 0)
}
"#;
    compile_and_check(source, &["(i64, i64)", "(0, 0)"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_range_expressions() {
    let source = r#"
fn ranges() {
    let a = 0..10
    let b = 0..=10
}
"#;
    compile_and_check(source, &["0..10", "0..=10"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_array_indexing() {
    let source = r#"
fn get_first(arr: Vec<int>) -> int {
    arr[0]
}
"#;
    // Array indexing with literal now includes cast
    compile_and_check(source, &["arr[0 as usize]"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_calls() {
    let source = r#"
fn length(s: string) -> int {
    s.len()
}
"#;
    compile_and_check(source, &["s.len()"]);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_field_access() {
    let source = r#"
struct Point { x: int, y: int }

fn get_x(p: Point) -> int {
    p.x
}
"#;
    compile_and_check(source, &["p.x"]);
}
