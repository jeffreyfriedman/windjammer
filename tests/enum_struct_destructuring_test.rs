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

// Test: Enum struct variant destructuring in match expressions
// Required for type-specific logic in editor panels

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_struct_wildcard() {
    let code = r#"
    enum Shape {
        Circle { radius: f32 },
        Rectangle { width: f32, height: f32 },
    }
    
    fn get_type(shape: &Shape) -> string {
        match shape {
            Shape::Circle { .. } => "circle",
            Shape::Rectangle { .. } => "rectangle",
        }
    }
    
    fn main() {
        let c = Shape::Circle { radius: 5.0 }
        let r = Shape::Rectangle { width: 10.0, height: 20.0 }
        println!("{}", get_type(&c))
        println!("{}", get_type(&r))
    }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Should compile: {:?}", result);

    let output = result.unwrap();
    assert!(
        output.contains("Shape::Circle { .. }"),
        "Should destructure Circle with wildcard: {}",
        output
    );
    assert!(
        output.contains("Shape::Rectangle { .. }"),
        "Should destructure Rectangle with wildcard: {}",
        output
    );
    test_utils::verify_rust_compiles(&output).expect("Generated Rust should compile");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_struct_extract_fields() {
    let code = r#"
    enum Shape {
        Circle { radius: f32 },
        Rectangle { width: f32, height: f32 },
    }
    
    fn area(shape: &Shape) -> f32 {
        match shape {
            Shape::Circle { radius } => 3.14159 * radius * radius,
            Shape::Rectangle { width, height } => width * height,
        }
    }
    
    fn main() {
        let c = Shape::Circle { radius: 2.0 }
        let r = Shape::Rectangle { width: 5.0, height: 3.0 }
        println!("{:.2}", area(&c))
        println!("{:.2}", area(&r))
    }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Should compile: {:?}", result);

    let output = result.unwrap();
    assert!(
        output.contains("Shape::Circle { radius }"),
        "Should destructure Circle with radius field: {}",
        output
    );
    assert!(
        output.contains("Shape::Rectangle { width, height }"),
        "Should destructure Rectangle with width and height: {}",
        output
    );
    assert!(
        output.contains("radius * radius"),
        "Should use radius in calculation: {}",
        output
    );
    test_utils::verify_rust_compiles(&output).expect("Generated Rust should compile");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_struct_partial_extract() {
    let code = r#"
    enum Light {
        Point { color: string, intensity: f32, range: f32 },
        Directional { color: string, intensity: f32 },
    }
    
    fn get_intensity(light: &Light) -> f32 {
        match light {
            Light::Point { intensity, .. } => *intensity,
            Light::Directional { intensity, .. } => *intensity,
        }
    }
    
    fn main() {
        let p = Light::Point { color: "red", intensity: 1.5, range: 10.0 }
        let d = Light::Directional { color: "white", intensity: 2.0 }
        println!("{:.1}", get_intensity(&p))
        println!("{:.1}", get_intensity(&d))
    }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Should compile: {:?}", result);

    let output = result.unwrap();
    assert!(
        output.contains("Light::Point { intensity, .. }"),
        "Should destructure Point with partial fields: {}",
        output
    );
    assert!(
        output.contains("Light::Directional { intensity, .. }"),
        "Should destructure Directional with partial fields: {}",
        output
    );
    test_utils::verify_rust_compiles(&output).expect("Generated Rust should compile");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_mixed_variants() {
    let code = r#"
    enum Value {
        Int(i32),
        Float(f32),
        Pair { x: i32, y: i32 },
    }
    
    fn describe(val: &Value) -> string {
        match val {
            Value::Int(n) => format!("int: {}", n),
            Value::Float(f) => format!("float: {:.1}", f),
            Value::Pair { x, y } => format!("pair: ({}, {})", x, y),
        }
    }
    
    fn main() {
        let i = Value::Int(42)
        let f = Value::Float(3.14)
        let p = Value::Pair { x: 10, y: 20 }
        println!("{}", describe(&i))
        println!("{}", describe(&f))
        println!("{}", describe(&p))
    }
    "#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Should compile: {:?}", result);

    let output = result.unwrap();
    assert!(
        output.contains("Value::Int(n)"),
        "Should destructure Int tuple variant: {}",
        output
    );
    assert!(
        output.contains("Value::Float(f)"),
        "Should destructure Float tuple variant: {}",
        output
    );
    assert!(
        output.contains("Value::Pair { x, y }"),
        "Should destructure Pair struct variant: {}",
        output
    );
    test_utils::verify_rust_compiles(&output).expect("Generated Rust should compile");
}
