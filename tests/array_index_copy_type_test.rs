// TDD Test: Compiler should NOT add &mut when indexing array of Copy types
// WINDJAMMER PHILOSOPHY: Copy types should be copied, not borrowed

use std::fs;
use std::process::Command;

fn compile_code(code: &str, test_name: &str) -> Result<String, String> {
    let test_dir = format!("tests/generated/array_index_copy_test_{}", test_name);
    fs::create_dir_all(&test_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test.wj", test_dir);
    fs::write(&input_file, code).expect("Failed to write source file");

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
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_file = format!("{}/test.rs", &test_dir);
    let generated = fs::read_to_string(&generated_file).expect("Failed to read generated file");

    fs::remove_dir_all(&test_dir).ok();

    Ok(generated)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_array_index_of_copy_type_should_not_add_mut_ref() {
    // BUG: Compiler incorrectly adds &mut when reading Copy type from array
    let code = r#"
    pub struct Storage {
        pub data: Vec<i64>,
    }
    
    impl Storage {
        pub fn get(&self, index: int) -> i64 {
            let value = self.data[index as usize]
            return value
        }
    }
    "#;

    let generated = compile_code(code, "no_mut_ref").expect("Compilation failed");

    // Should NOT add &mut for Copy types
    assert!(
        !generated.contains("&mut self.data["),
        "Should NOT add &mut when indexing Copy type array, got:\n{}",
        generated
    );

    // Should be a simple read
    assert!(
        generated.contains("self.data[index as usize]")
            || generated.contains("let value = self.data["),
        "Should be a simple array read, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_inline_array_index_copy_type_in_return() {
    // Real case from entity.rs
    let code = r#"
    pub struct Manager {
        pub generations: Vec<i64>,
    }
    
    pub struct Entity {
        pub id: i64,
        pub gen: i64,
    }
    
    impl Manager {
        pub fn create(&self, index: i64) -> Entity {
            let generation = self.generations[index as usize]
            return Entity { id: index, gen: generation }
        }
    }
    "#;

    let generated = compile_code(code, "inline_return").expect("Compilation failed");

    // Should NOT have &mut for Copy type
    assert!(
        !generated.contains("&mut self.generations["),
        "Should NOT add &mut when reading Copy type, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_array_index_non_copy_type_may_add_ref() {
    // For non-Copy types, borrowing might be correct
    let code = r#"
    pub struct Item {
        pub name: string,
    }
    
    pub fn get_item(items: Vec<Item>, index: usize) -> string {
        let item = items[index]
        return item.name
    }
    "#;

    let generated = compile_code(code, "non_copy").expect("Compilation failed");

    // For non-Copy types, & is appropriate (or .clone())
    // This test just verifies compilation succeeds
    assert!(
        generated.contains("fn get_item"),
        "Should compile successfully"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_copy_type_used_in_function_call() {
    let code = r#"
    pub fn process(id: i64) -> i64 {
        return id * 2
    }
    
    pub fn run(ids: Vec<i64>, index: usize) -> i64 {
        let id = ids[index]
        return process(id)
    }
    "#;

    let generated = compile_code(code, "function_call").expect("Compilation failed");

    // Should pass by value, not reference
    assert!(
        !generated.contains("&mut ids["),
        "Should NOT add &mut for Copy type in array index, got:\n{}",
        generated
    );
}
