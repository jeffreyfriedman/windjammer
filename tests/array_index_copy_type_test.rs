// TDD Test: Compiler should NOT add &mut when indexing array of Copy types
// WINDJAMMER PHILOSOPHY: Copy types should be copied, not borrowed

#[path = "test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_array_index_of_copy_type_should_not_add_mut_ref() {
    // BUG: Compiler incorrectly adds &mut when reading Copy type from array
    let code = r#"
    pub struct Storage {
        pub data: Vec<i64>,
    }
    
    impl Storage {
        pub fn get(self, index: usize) -> i64 {
            let value = self.data[index]
            return value
        }
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

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
        pub id: usize,
        pub gen: i64,
    }
    
    impl Manager {
        pub fn create(self, index: usize) -> Entity {
            let generation = self.generations[index]
            return Entity { id: index, gen: generation }
        }
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

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

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

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

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should pass by value, not reference
    assert!(
        !generated.contains("&mut ids["),
        "Should NOT add &mut for Copy type in array index, got:\n{}",
        generated
    );
}
