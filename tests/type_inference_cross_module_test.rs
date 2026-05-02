/// TDD Test: Cross-Module Type Inference
///
/// Pattern: Function calls across modules should propagate type information
/// Example: mod math { pub fn distance(a: f32, b: f32) -> f32 }
///          Using: let d = math::distance(self.x, 0.0)
///          Should infer: 0.0 as f32 based on function signature
///
/// This tests whether the compiler can look up function signatures
/// from other modules and use them for type inference.
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_cross_module_function_call() {
    let results = test_utils::compile_project(&[
        (
            "math.wj",
            r#"
pub fn distance(a: f32, b: f32) -> f32 {
    let dx = a - b
    dx * dx
}
"#,
        ),
        (
            "main.wj",
            r#"
use math

struct Point {
    x: f32,
    y: f32,
}

impl Point {
    pub fn distance_from_origin(self) -> f32 {
        math::distance(self.x, 0.0)
    }
}
"#,
        ),
    ]);

    let output = results.get("main.rs").expect("main.rs not generated");

    assert!(
        output.contains("0.0_f32") || !output.contains("0.0_f64"),
        "0.0 should be f32, not f64: {}",
        output
    );
}

#[test]
fn test_cross_module_struct_field_access() {
    let results = test_utils::compile_project(&[
        (
            "types.wj",
            r#"
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}
"#,
        ),
        (
            "main.wj",
            r#"
use types

pub fn calculate(v: Vector2) -> f32 {
    v.x + 0.5
}
"#,
        ),
    ]);

    let output = results.get("main.rs").expect("main.rs not generated");

    assert!(
        output.contains("0.5_f32") || !output.contains("0.5_f64"),
        "0.5 should be f32: {}",
        output
    );
}

#[test]
fn test_cross_module_method_call() {
    let results = test_utils::compile_project(&[
        (
            "math.wj",
            r#"
pub struct Calculator {
    pub factor: f32,
}

impl Calculator {
    pub fn multiply(self, value: f32) -> f32 {
        self.factor * value
    }
}
"#,
        ),
        (
            "main.wj",
            r#"
use math

pub fn calculate(calc: Calculator) -> f32 {
    calc.multiply(2.0)
}
"#,
        ),
    ]);

    let output = results.get("main.rs").expect("main.rs not generated");

    assert!(
        output.contains("2.0_f32") || !output.contains("2.0_f64"),
        "2.0 should be f32: {}",
        output
    );
}
