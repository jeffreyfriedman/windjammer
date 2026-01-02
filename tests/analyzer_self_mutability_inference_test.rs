// TDD Test: Analyzer should infer &mut self when needed
// THE WINDJAMMER WAY: Compiler infers mutability, developer writes clean code

use std::fs;
use std::process::Command;

fn compile_and_check_rust(code: &str, test_name: &str) -> Result<String, String> {
    let test_dir = format!("tests/generated/self_mutability_{}", test_name);
    fs::create_dir_all(&test_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test.wj", test_dir);
    fs::write(&input_file, code).expect("Failed to write source file");

    // Compile with Windjammer
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            &input_file,
            "--output",
            &test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        fs::remove_dir_all(&test_dir).ok();
        return Err(format!(
            "Windjammer compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Check generated Rust compiles
    let rust_check = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            &format!("{}/test.rs", test_dir),
            "-o",
            &format!("{}/test.rlib", test_dir),
        ])
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&rust_check.stderr).to_string();

    fs::remove_dir_all(&test_dir).ok();

    if rust_check.status.success() {
        Ok(String::new())
    } else {
        Err(stderr)
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_returning_mut_ref_needs_mut_self() {
    // Method returning &mut T should have &mut self
    let code = r#"
    pub struct Storage<T> {
        pub data: Vec<T>,
    }
    
    impl<T> Storage<T> {
        pub fn get_mut(self, index: usize) -> Option<&mut T> {
            if index < self.data.len() {
                return Some(&mut self.data[index])
            }
            return None
        }
    }
    "#;

    let result = compile_and_check_rust(code, "returns_mut_ref");

    assert!(
        result.is_ok(),
        "Method returning &mut T should infer &mut self, got error:\n{}",
        result.err().unwrap()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_mutating_field_needs_mut_self() {
    // Method that mutates a field should have &mut self
    let code = r#"
    pub struct Counter {
        pub count: i32,
    }
    
    impl Counter {
        pub fn increment(self) {
            self.count = self.count + 1
        }
    }
    "#;

    let result = compile_and_check_rust(code, "mutates_field");

    assert!(
        result.is_ok(),
        "Method mutating field should infer &mut self, got error:\n{}",
        result.err().unwrap()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_calling_mutating_method_needs_mut_self() {
    // Method that calls another mutating method should have &mut self
    let code = r#"
    pub struct Emitter {
        pub count: i32,
    }
    
    impl Emitter {
        pub fn emit_particle(self) {
            self.count = self.count + 1
        }
        
        pub fn burst(self, amount: i32) {
            for i in 0..amount {
                self.emit_particle()
            }
        }
    }
    "#;

    let result = compile_and_check_rust(code, "calls_mutating");

    assert!(
        result.is_ok(),
        "Method calling mutating method should infer &mut self, got error:\n{}",
        result.err().unwrap()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_read_only_method_uses_ref_self() {
    // Method that only reads should have &self (not &mut self)
    let code = r#"
    pub struct Point {
        pub x: f32,
        pub y: f32,
    }
    
    impl Point {
        pub fn distance_from_origin(self) -> f32 {
            return (self.x * self.x + self.y * self.y).sqrt()
        }
    }
    "#;

    let result = compile_and_check_rust(code, "read_only");

    assert!(
        result.is_ok(),
        "Read-only method should use &self, got error:\n{}",
        result.err().unwrap()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_compound_assignment_needs_mut_self() {
    // Method using compound assignment (+=) should have &mut self
    let code = r#"
    pub struct Score {
        pub points: i32,
    }
    
    impl Score {
        pub fn add_points(self, amount: i32) {
            self.points += amount
        }
    }
    "#;

    let result = compile_and_check_rust(code, "compound_assign");

    assert!(
        result.is_ok(),
        "Method with compound assignment should infer &mut self, got error:\n{}",
        result.err().unwrap()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_mutating_vec_needs_mut_self() {
    // Method calling Vec::push should have &mut self
    let code = r#"
    pub struct List {
        pub items: Vec<i32>,
    }
    
    impl List {
        pub fn add(self, item: i32) {
            self.items.push(item)
        }
    }
    "#;

    let result = compile_and_check_rust(code, "vec_push");

    assert!(
        result.is_ok(),
        "Method calling Vec::push should infer &mut self, got error:\n{}",
        result.err().unwrap()
    );
}
