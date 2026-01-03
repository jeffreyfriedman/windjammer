// Test: Enum struct variant destructuring in match expressions
// Required for type-specific logic in editor panels

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_run(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let src_file = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::create_dir(&out_dir).map_err(|e| format!("Failed to create out dir: {}", e))?;

    let mut file =
        fs::File::create(&src_file).map_err(|e| format!("Failed to create source file: {}", e))?;
    file.write_all(code.as_bytes())
        .map_err(|e| format!("Failed to write source: {}", e))?;

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    // Compile
    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(&src_file)
        .arg("-o")
        .arg(&out_dir)
        .output()
        .map_err(|e| format!("Failed to run wj: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Run the compiled binary (it's in target/debug/ after cargo build)
    let binary = out_dir.join("target").join("debug").join("test");
    let run_output = Command::new(&binary)
        .output()
        .map_err(|e| format!("Failed to run binary: {}", e))?;

    if !run_output.status.success() {
        return Err(format!(
            "Execution failed:\n{}",
            String::from_utf8_lossy(&run_output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&run_output.stdout).to_string())
}

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

    let result = compile_and_run(code);
    assert!(result.is_ok(), "Should compile: {:?}", result);

    let output = result.unwrap();
    assert!(
        output.contains("circle"),
        "Should print 'circle': {}",
        output
    );
    assert!(
        output.contains("rectangle"),
        "Should print 'rectangle': {}",
        output
    );
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

    let result = compile_and_run(code);
    assert!(result.is_ok(), "Should compile: {:?}", result);

    let output = result.unwrap();
    assert!(
        output.contains("12.57"),
        "Circle area should be ~12.57: {}",
        output
    );
    assert!(
        output.contains("15.00"),
        "Rectangle area should be 15.00: {}",
        output
    );
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

    let result = compile_and_run(code);
    assert!(result.is_ok(), "Should compile: {:?}", result);

    let output = result.unwrap();
    assert!(output.contains("1.5"), "Point intensity: {}", output);
    assert!(output.contains("2.0"), "Directional intensity: {}", output);
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

    let result = compile_and_run(code);
    assert!(result.is_ok(), "Should compile: {:?}", result);

    let output = result.unwrap();
    assert!(output.contains("int: 42"), "Int variant: {}", output);
    assert!(output.contains("float: 3.1"), "Float variant: {}", output);
    assert!(
        output.contains("pair: (10, 20)"),
        "Pair variant: {}",
        output
    );
}
