#[cfg(test)]
mod ownership_inference_method_self_by_value {

    #[test]
    fn test_method_self_by_value_should_not_infer_mut() {
        let source = r#"
struct Point {
    x: f32,
    y: f32
}

impl Point {
    fn double(self) -> Point {
        Point {
            x: self.x * 2.0,
            y: self.y * 2.0
        }
    }
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 }
    let p2 = p.double()
    
    assert(p2.x == 2.0, "x should be doubled")
    assert(p2.y == 4.0, "y should be doubled")
}
"#;

        let program = parse_and_analyze(source);
        assert!(program.is_ok(), "Should compile without mutability errors");
        
        let rust_code = generate_rust(&program.unwrap());
        
        // Should NOT contain "let mut p"
        assert!(!rust_code.contains("let mut p = Point"), 
            "Should not infer 'mut' for variable passed to method with self by value");
        
        // Method signature should be "self" not "&mut self"
        assert!(rust_code.contains("fn double(self)"), 
            "Method should take 'self' by value, not '&mut self'");
    }

    #[test]
    fn test_method_self_by_value_with_multiply() {
        let source = r#"
struct Mat4 {
    m00: f32, m11: f32, m22: f32, m33: f32
}

impl Mat4 {
    fn identity() -> Mat4 {
        Mat4 { m00: 1.0, m11: 1.0, m22: 1.0, m33: 1.0 }
    }
    
    fn multiply(self, other: Mat4) -> Mat4 {
        Mat4 {
            m00: self.m00 * other.m00,
            m11: self.m11 * other.m11,
            m22: self.m22 * other.m22,
            m33: self.m33 * other.m33
        }
    }
}

fn main() {
    let identity = Mat4::identity()
    let result = identity.multiply(identity)
    
    assert(result.m00 == 1.0, "Should work")
}
"#;

        let program = parse_and_analyze(source);
        assert!(program.is_ok(), "Should compile without mutability errors: {:?}", 
            program.as_ref().err());
        
        let rust_code = generate_rust(&program.unwrap());
        
        // Should NOT require mut
        assert!(!rust_code.contains("let mut identity"), 
            "Should not infer 'mut' for immutable variable");
    }

    fn parse_and_analyze(_source: &str) -> Result<Program, String> {
        // This is a placeholder - actual implementation would use the real parser/analyzer
        Err("Not implemented".to_string())
    }

    fn generate_rust(_program: &Program) -> String {
        String::new()
    }

    struct Program;
}
