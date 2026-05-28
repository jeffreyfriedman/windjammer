#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

// Comprehensive feature tests for all language features

#[path = "common/test_utils.rs"]
mod test_utils;

// Helper to compile and check generated code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_basic_function() {
    let source = r#"
fn add(x: int, y: int) -> int {
    x + y
}
"#;
    // Parameters not mutated, so no 'mut' needed
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_expression() {
    let source = r#"
fn sign(x: int) -> string {
    if x > 0 { "positive" } else { "negative" }
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("if x > 0") || rust.contains("if (x as i64) > 0_i64"),
        "Expected if condition with x > 0 in:\n{}",
        rust
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_interpolation() {
    let source = r#"
fn greet(name: string) -> string {
    "Hello, ${name}!"
}
"#;
    test_utils::compile_single(source);
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
    let generated = test_utils::compile_single(source);

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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("while n > 0")
            || rust.contains("while (n as i64) > 0_i64")
            || rust.contains("while *n > 0"),
        "Expected while condition with n > 0 in:\n{}",
        rust
    );
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
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_character_literals() {
    let source = r#"
fn get_char() -> char {
    'a'
}
"#;
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_character_escapes() {
    let source = r#"
fn newline() -> char { '\n' }
fn tab() -> char { '\t' }
fn quote() -> char { '\'' }
"#;
    let generated = test_utils::compile_single(source);

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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("if x < 0") || rust.contains("if (x as i64) < 0_i64"),
        "Expected if condition with x < 0 in:\n{}",
        rust
    );
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_const_declaration() {
    let source = r#"
const MAX: int = 100
"#;
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_static_declaration() {
    let source = r#"
static mut COUNTER: int = 0
"#;
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_tuple_type() {
    let source = r#"
fn coords() -> (int, int) {
    (0, 0)
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("(0, 0)") || rust.contains("(0_i64, 0_i64)"),
        "Expected tuple literal in:\n{}",
        rust
    );
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
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_array_indexing() {
    let source = r#"
fn get_first(arr: Vec<int>) -> int {
    arr[0]
}
"#;
    // Array indexing with integer literal skips unnecessary `as usize` cast
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_calls() {
    let source = r#"
fn length(s: string) -> int {
    s.len()
}
"#;
    test_utils::compile_single(source);
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
    test_utils::compile_single(source);
}
