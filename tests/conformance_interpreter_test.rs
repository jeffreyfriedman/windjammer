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

/// Cross-backend conformance: enums, guards, strings, collections.
#[path = "cross_backend_conformance_harness.rs"]
mod cross_backend_conformance_harness;
use cross_backend_conformance_harness::assert_backends_agree;

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
