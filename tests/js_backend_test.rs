/// Integration tests for the JavaScript backend (TDD)
///
/// Tests impl block → class method generation, match expressions,
/// and ensures the JS backend produces valid Node.js-executable code.
#[path = "test_utils.rs"]
mod test_utils;

/// Compile .wj source to JavaScript and return the generated JS code
/// Compile .wj to JS and run with Node.js. Returns stdout.
// ==========================================
// Basic JS generation tests
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_js_hello_world() {
    let output = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let code = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let code = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let code = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let code = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
    let output = test_utils::compile_single(
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
