/// TDD: Method return type in binary operations (E0308 bug)
///
/// **Bug**: Context inference doesn't propagate through method call return types.
///
/// **Pattern (from ai/astar_grid.wj):**
/// ```windjammer
/// fn get_cost() -> f32 { ... }
/// let x = self.get_cost(1, 2) * 1.414  // 1.414 should be f32, gets f64!
/// ```
///
/// **Root cause**: Binary operation inference doesn't check method call return types.
///
/// **Proper fix**: Extend context inference to handle method_call * literal.

use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    if !float_inference.errors.is_empty() {
        panic!("Float inference errors: {:?}", float_inference.errors);
    }

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

/// method_call() -> f32, then method_call() * literal should infer literal as f32
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

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
    // Both 1.0 and 2.0 should be f32
    assert!(
        (output.contains("1.0_f32") || output.contains("1_f32")) &&
        (output.contains("2.0_f32") || output.contains("2_f32")),
        "Both literals in (self.get() + 1.0) * 2.0 should be f32:\n{}",
        output
    );
}
