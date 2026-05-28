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

// TDD Test: Analyzer should infer &mut self when needed
// THE WINDJAMMER WAY: Compiler infers mutability, developer writes clean code

#[path = "common/test_utils.rs"]
mod test_utils;

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

    let result = test_utils::compile_single_result(code);

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

    let result = test_utils::compile_single_result(code);

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

    let result = test_utils::compile_single_result(code);

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

    let result = test_utils::compile_single_result(code);

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

    let result = test_utils::compile_single_result(code);

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

    let result = test_utils::compile_single_result(code);

    assert!(
        result.is_ok(),
        "Method calling Vec::push should infer &mut self, got error:\n{}",
        result.err().unwrap()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_mutating_hashmap_needs_mut_self() {
    // HashMap::insert also requires &mut
    let code = r#"
    use std::collections::HashMap
    
    pub struct Cache {
        pub data: HashMap<string, i32>,
    }
    
    impl Cache {
        pub fn store(self, key: string, value: i32) {
            self.data.insert(key, value)
        }
    }
    "#;

    let result = test_utils::compile_single_result(code);

    assert!(
        result.is_ok(),
        "Method calling HashMap::insert should infer &mut self, got error:\n{}",
        result.err().unwrap()
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_field_mutation_needs_mut_self() {
    // Nested: self.player.position.x += ...
    let code = r#"
    pub struct Position {
        pub x: f32,
        pub y: f32,
    }
    
    pub struct Player {
        pub position: Position,
    }
    
    pub struct Game {
        pub player: Player,
    }
    
    impl Game {
        pub fn move_player(self, dx: f32) {
            self.player.position.x += dx
        }
    }
    "#;

    let result = test_utils::compile_single_result(code);

    assert!(
        result.is_ok(),
        "Method with nested field mutation should infer &mut self, got error:\n{}",
        result.err().unwrap()
    );
}
