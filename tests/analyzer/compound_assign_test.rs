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

/// TDD Test: Compound assignment optimization for field access patterns
///
/// Bug: `self.x = self.x + dt` generates as-is instead of `self.x += dt`.
/// The compound assignment optimization only handled simple identifiers (x = x + y),
/// not field access patterns (self.x = self.x + y) or index patterns (arr[i] = arr[i] + 1).
///
/// Fix: Extended the pattern matcher to detect FieldAccess and Index targets.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_field_compound_add() {
    let source = r#"
pub struct Timer {
    pub elapsed: f32,
}

impl Timer {
    pub fn tick(&mut self, dt: f32) {
        self.elapsed = self.elapsed + dt
    }
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("self.elapsed += dt"),
        "self.x = self.x + y should become self.x += y.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_field_compound_sub() {
    let source = r#"
pub struct Health {
    pub hp: i32,
}

impl Health {
    pub fn damage(&mut self, amount: i32) {
        self.hp = self.hp - amount
    }
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("self.hp -= amount"),
        "self.x = self.x - y should become self.x -= y.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_field_compound_mul() {
    let source = r#"
pub struct Transform {
    pub scale: f32,
}

impl Transform {
    pub fn scale_by(&mut self, factor: f32) {
        self.scale = self.scale * factor
    }
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("self.scale *= factor"),
        "self.x = self.x * y should become self.x *= y.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_simple_var_compound_still_works() {
    let source = r#"
pub fn accumulate(n: i32) -> i32 {
    let mut total = 0
    let mut i = 0
    total = total + n
    total
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("total += n"),
        "Simple x = x + y should still be converted to x += y.\nGenerated:\n{}",
        generated
    );
}
