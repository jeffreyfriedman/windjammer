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

/// Cross-backend conformance: composition, logic, nested loops, structs.
#[path = "cross_backend_conformance_harness.rs"]
mod cross_backend_conformance_harness;
use cross_backend_conformance_harness::assert_backends_agree;

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
