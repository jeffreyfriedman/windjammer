#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// method_call() -> f32, then method_call() * literal should infer literal as f32

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_method_return_f32_in_binary_op() {
    let source = r#"
pub struct Grid {
    pub cells: Vec<f32>,
}

impl Grid {
    pub fn get_cost(self, x: i32, y: i32) -> f32 {
        self.cells[0]
    }
    
    pub fn diagonal_cost(self, x: i32, y: i32) -> f32 {
        self.get_cost(x, y) * 1.414
    }
}
"#;

    let output = test_utils::compile_single(source);

    // 1.414 should be inferred as f32 from get_cost() return type
    assert!(
        output.contains("1.414_f32") || output.contains("1.414f32"),
        "1.414 in self.get_cost() * 1.414 should be f32 (get_cost returns f32):\n{}",
        output
    );

    assert!(
        !output.contains("1.414_f64"),
        "Should not generate f64 when multiplying with f32 method result:\n{}",
        output
    );
}

/// function_call() -> f32, then function_call() + literal should infer literal as f32
#[test]
fn test_function_return_f32_in_binary_op() {
    let source = r#"
pub fn get_value() -> f32 {
    42.0
}

pub fn compute() -> f32 {
    get_value() + 3.14
}
"#;

    let output = test_utils::compile_single(source);

    // 3.14 should be f32 from get_value() return type
    assert!(
        output.contains("3.14_f32") || output.contains("3.14f32"),
        "3.14 in get_value() + 3.14 should be f32:\n{}",
        output
    );
}

/// EXACT pattern from ai/astar_grid.wj: AStarGrid.get_cost returns f32
#[test]
fn test_astar_grid_get_cost_times_literal() {
    let source = r#"
pub struct AStarCell {
    pub walkable: bool,
    pub cost: f32,
}

pub struct AStarGrid {
    pub width: i32,
    pub height: i32,
    pub cells: Vec<AStarCell>,
    pub allow_diagonal: bool,
}

impl AStarGrid {
    fn index(self, x: i32, y: i32) -> i32 {
        y * self.width + x
    }
    
    fn get_cost(self, x: i32, y: i32) -> f32 {
        self.cells[0].cost
    }
    
    fn get_neighbors(self, x: i32, y: i32) -> Vec<(i32, i32, f32)> {
        let mut result = Vec::new()
        if self.allow_diagonal {
            result.push((x + 1, y + 1, self.get_cost(x + 1, y + 1) * 1.414))
        }
        result
    }
}
"#;

    let output = test_utils::compile_single(source);

    // Exact bug: self.get_cost(x+1, y+1) * 1.414 must generate 1.414_f32
    assert!(
        output.contains("1.414_f32") || output.contains("1.414f32"),
        "1.414 in self.get_cost() * 1.414 must be f32 (AStarGrid pattern):\n{}",
        output
    );
    assert!(
        !output.contains("1.414_f64"),
        "Should NOT generate f64 when multiplying with f32 get_cost result:\n{}",
        output
    );
}

/// Complex: (method_call() + literal) * literal - both literals should be f32
#[test]
fn test_chained_binary_ops_with_method() {
    let source = r#"
pub struct State {
    pub value: f32,
}

impl State {
    pub fn get(self) -> f32 {
        self.value
    }
    
    pub fn compute(self) -> f32 {
        (self.get() + 1.0) * 2.0
    }
}
"#;

    let output = test_utils::compile_single(source);

    // Both 1.0 and 2.0 should be f32
    assert!(
        (output.contains("1.0_f32") || output.contains("1_f32"))
            && (output.contains("2.0_f32") || output.contains("2_f32")),
        "Both literals in (self.get() + 1.0) * 2.0 should be f32:\n{}",
        output
    );
}
