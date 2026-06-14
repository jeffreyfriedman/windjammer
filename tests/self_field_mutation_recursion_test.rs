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

//! TDD tests for `&mut self` / mutation analysis without stack overflow.
//!
//! Covers:
//! - Nested match/if/block shapes in `self_analysis`
//! - Mutual recursion between `impl` methods (`a` → `b` → `a`): `HashSet` of method names in
//!   `function_modifies_self_fields_recursive` breaks the cycle (conservative: treat as mutating).
//! - `return self.cells[i].touch()` where `touch` is only known mutating via registry / same-file `impl`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_self_field_mutation_in_nested_match() {
    // Minimal reproduction: method with match arm containing self.field mutation
    let src = r#"
pub struct Game {
    score: i32,
    lives: i32,
}

impl Game {
    pub fn update(self, event: i32) {
        match event {
            1 => {
                self.score = self.score + 10
            },
            2 => {
                self.lives = self.lives - 1
            },
            _ => {}
        }
    }
}
"#;

    let result = test_utils::compile_single_result(src);

    // Should NOT stack overflow
    assert!(result.is_ok(), "Should compile without stack overflow");

    let rust = result.unwrap();

    // Should infer &mut self (self.score and self.lives are mutated)
    assert!(
        rust.contains("pub fn update(&mut self, event: i32)"),
        "Should infer &mut self for mutating method\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_self_method_call_in_match_arm() {
    let src = r#"
pub struct Player {
    health: f32,
}

impl Player {
    pub fn take_damage(self, amount: f32) {
        self.health = self.health - amount
    }

    pub fn process_event(self, event: i32) {
        match event {
            1 => self.take_damage(10.0),
            2 => self.take_damage(5.0),
            _ => {}
        }
    }
}
"#;

    let result = test_utils::compile_single_result(src);

    assert!(result.is_ok(), "Should compile without stack overflow");

    let rust = result.unwrap();

    // take_damage mutates self.health, so needs &mut self
    assert!(rust.contains("pub fn take_damage(&mut self, amount: f32)"));

    // process_event calls take_damage, so also needs &mut self
    assert!(rust.contains("pub fn process_event(&mut self, event: i32)"));
}

#[test]
fn test_deeply_nested_match_in_if() {
    let src = r#"
pub struct State {
    value: i32,
}

impl State {
    pub fn update(self, a: i32, b: i32) {
        if a > 0 {
            match b {
                1 => { self.value = 10 },
                2 => { self.value = 20 },
                _ => { self.value = 0 }
            }
        }
    }
}
"#;

    let result = test_utils::compile_single_result(src);

    assert!(result.is_ok(), "Should compile without stack overflow");

    let rust = result.unwrap();
    assert!(rust.contains("pub fn update(&mut self, a: i32, b: i32)"));
}

/// Mutual `impl` recursion (`a` → `b` → `a`) used to blow the stack when
/// `function_modifies_self_fields_recursive` re-entered the same callee without a guard.
#[test]
fn test_mutually_recursive_self_methods_no_stack_overflow() {
    let src = r#"
pub struct Node {}

impl Node {
    pub fn a(self) {
        self.b()
    }
    pub fn b(self) {
        self.a()
    }
}
"#;

    let result = test_utils::compile_single_result(src);
    assert!(
        result.is_ok(),
        "mutual recursion in self-call analysis must not stack overflow: {:?}",
        result.as_ref().err()
    );
}

/// Real-world shape: `return` in a branch with indexed field + callee not in `is_mutating_method` heuristics.
#[test]
fn test_return_indexed_self_field_custom_mut_method() {
    let src = r#"
pub struct Cell {
    values: Vec<i32>,
}

impl Cell {
    pub fn touch(self) {
        self.values.push(1)
    }
}

pub struct Grid {
    cells: Vec<Cell>,
}

impl Grid {
    pub fn run(self, idx: i32) -> bool {
        if idx == 0 {
            return self.cells[idx].touch()
        }
        false
    }
}
"#;

    let result = test_utils::compile_single_result(src);
    assert!(result.is_ok(), "compile: {:?}", result.as_ref().err());
    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn run(&mut self,") || rust.contains("pub fn run(&mut self ,"),
        "expected &mut self for return self.cells[i].touch(); got:\n{}",
        rust
    );
}
