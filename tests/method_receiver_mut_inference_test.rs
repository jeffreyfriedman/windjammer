//! TDD: E0596-style mut receiver inference
//!
//! 1. Parameters used only inside `match` arms with mutating callee methods need `&mut`.
//! 2. `self.vec[i].other_type_method()` must infer `&mut self` when callee's method is `&mut self`
//!    (qualified registry lookup on the element type).

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_match_arm_parameter_mut_when_callee_mutates() {
    let src = r#"
pub struct Cell {
    values: Vec<i32>,
}

impl Cell {
    pub fn bump(self) {
        self.values.push(1)
    }
}

pub enum Tag {
    A,
}

impl Tag {
    pub fn run(self, c: Cell) {
        match self {
            Tag::A => {
                c.bump()
            }
        }
    }
}
"#;

    let rust = test_utils::compile_single_result(src).unwrap_or_else(|e| panic!("compile: {}", e));
    assert!(
        rust.contains("pub fn run(&self, c: &mut Cell)"),
        "expected `c: &mut Cell` for mutating calls inside match arms; got:\n{}",
        rust
    );
}

#[test]
fn test_return_in_if_propagates_mutating_call_on_indexed_field() {
    let src = r#"
pub struct Cell {
    values: Vec<i32>,
}

impl Cell {
    pub fn touch() {
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

    let rust = test_utils::compile_single_result(src).unwrap_or_else(|e| panic!("compile: {}", e));
    assert!(
        rust.contains("pub fn run(&mut self,") || rust.contains("pub fn run(&mut self ,"),
        "expected &mut self when return ... self.cells[i].touch() in branch; got:\n{}",
        rust
    );
}

#[test]
fn test_indexed_self_field_element_mut_method_requires_mut_self() {
    let src = r#"
pub struct Inner {
    items: Vec<i32>,
}

impl Inner {
    pub fn add_one(self) {
        self.items.push(1)
    }
}

pub struct Outer {
    cells: Vec<Inner>,
}

impl Outer {
    pub fn tick(self) {
        self.cells[0].add_one()
    }
}
"#;

    let rust = test_utils::compile_single_result(src).unwrap_or_else(|e| panic!("compile: {}", e));
    assert!(
        rust.contains("pub fn tick(&mut self)"),
        "expected `&mut self` when calling mutating method on indexed field element; got:\n{}",
        rust
    );
}
