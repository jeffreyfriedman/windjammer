#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "conformance_tests",
))]

/// Cross-backend conformance: baseline programs and Rust+Interpreter-only cases.
#[path = "cross_backend_conformance_harness.rs"]
mod cross_backend_conformance_harness;
use cross_backend_conformance_harness::{assert_backends_agree, assert_rust_and_interpreter_agree};

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

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_enum_with_data() {
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
