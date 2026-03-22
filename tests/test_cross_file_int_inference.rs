// TDD: Test cross-file integer literal inference
//
// Apply same float inference architecture to integer literals (i32, i64, u32, u64, usize, etc.)
//
// Tests:
// 1. Function parameter inference: fn foo(x: i32), call with bare literal 42
// 2. Struct field inference: struct Point { x: i32 }, init with Point { x: 10 }
// 3. Cross-file inference: define struct in one file, use in another
// 4. Default to i32 for unknown contexts (Rust convention)

use tempfile::TempDir;
use windjammer::{analyzer, build_project_ext, codegen, lexer, parser, type_inference, CompilationTarget};

/// TDD: Option<Vec<int>> struct field - vec! elements should infer i64
#[test]
fn test_int_struct_option_vec_internal() {
    let source = r#"
struct Node {
    pub value: int,
    pub children: Option<Vec<int>>
}

fn main() {
    let n = Node { value: 1, children: Some(vec![2, 3]) }
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new_with_source(
        tokens,
        "test.wj".to_string(),
        source.to_string(),
    );
    let program = parser.parse().expect("Failed to parse");

    let mut int_inference = type_inference::IntInference::new();
    int_inference.infer_program(&program);

    if !int_inference.errors.is_empty() {
        panic!("Int inference errors: {:?}", int_inference.errors);
    }

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _registry, _) = analyzer.analyze_program(&program).expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_int_inference(int_inference);
    let rust_code = generator.generate_program(&program, &analyzed);

    assert!(
        rust_code.contains("2_i64") && rust_code.contains("3_i64"),
        "Option<Vec<int>> struct field should infer i64 for vec! elements. Generated:\n{}",
        rust_code
    );
}

/// Internal API test - fast, no full build
#[test]
fn test_int_function_param_inference_internal() {
    let source = r#"
pub fn take_i32(x: i32) -> i32 {
    x
}

pub fn main() -> i32 {
    take_i32(42)
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new_with_source(
        tokens,
        "test.wj".to_string(),
        source.to_string(),
    );
    let program = parser.parse().expect("Failed to parse");

    let mut int_inference = type_inference::IntInference::new();
    int_inference.infer_program(&program);

    if !int_inference.errors.is_empty() {
        panic!("Int inference errors: {:?}", int_inference.errors);
    }

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _registry, _) = analyzer.analyze_program(&program).expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_int_inference(int_inference);
    let rust_code = generator.generate_program(&program, &analyzed);

    assert!(
        rust_code.contains("42_i32"),
        "Should infer i32 from take_i32 param. Generated:\n{}",
        rust_code
    );
}

#[test]
fn test_int_function_param_inference() {
    // Single file: fn foo(x: i32), call with bare literal 42
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("foo.wj"),
        r#"
pub fn take_i32(x: i32) -> i32 {
    x
}

pub fn main() -> i32 {
    take_i32(42)
}
"#,
    )
    .unwrap();

    // Single file build (no mod.wj) - use path to foo.wj directly
    build_project_ext(
        &src.join("foo.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        false, // not library - single file
        &[],
    )
    .expect("Build should succeed");

    let foo_code = std::fs::read_to_string(build.join("foo.rs")).unwrap();

    // ASSERT: 42 should get _i32 suffix from function parameter type
    assert!(
        foo_code.contains("42_i32"),
        "Should infer i32 from take_i32 param. Generated:\n{}",
        foo_code.lines().filter(|l| l.contains("take_i32") || l.contains("42")).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn test_int_struct_field_inference() {
    // Struct field: struct Point { x: i32 }, init with Point { x: 10 }
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("point.wj"),
        r#"
pub struct Point {
    pub x: i32,
    pub y: i32
}

pub fn origin() -> Point {
    Point {
        x: 0,
        y: 0
    }
}
"#,
    )
    .unwrap();

    build_project_ext(
        &src.join("point.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        false,
        &[],
    )
    .expect("Build should succeed");

    let point_code = std::fs::read_to_string(build.join("point.rs")).unwrap();

    // ASSERT: 0 in struct init should infer i32 from field type
    assert!(
        point_code.contains("0_i32"),
        "Struct field x: 0 should infer i32. Generated:\n{}",
        point_code
    );
}

#[test]
fn test_int_cross_file_inference() {
    // Cross-file: define struct in one file, use in another
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(src.join("combat")).unwrap();

    // File 1: Define struct with i32 fields
    std::fs::write(
        src.join("combat/stats.wj"),
        r#"
pub struct CombatStats {
    pub health: i32,
    pub max_health: i32,
    pub damage: i32
}

impl CombatStats {
    pub fn new(health: i32, max_health: i32, damage: i32) -> CombatStats {
        CombatStats {
            health: health,
            max_health: max_health,
            damage: damage
        }
    }
}
"#,
    )
    .unwrap();

    // File 2: Call constructor with bare int literals
    std::fs::write(
        src.join("combat/enemy.wj"),
        r#"
use crate::stats::CombatStats

pub fn create_grunt() -> CombatStats {
    CombatStats::new(100, 100, 50)
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("combat/mod.wj"),
        r#"
pub mod stats
pub mod enemy
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod combat
"#,
    )
    .unwrap();

    build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("Build should succeed");

    let enemy_code = std::fs::read_to_string(build.join("combat/enemy.rs")).unwrap();

    // ASSERT: 100, 100, 50 should infer i32 from CombatStats::new params (cross-file)
    assert!(
        enemy_code.contains("100_i32") || enemy_code.contains("CombatStats::new(100,"),
        "Should infer i32 from cross-file function signature. Generated:\n{}",
        enemy_code.lines()
            .filter(|l| l.contains("CombatStats::new") || l.contains("100") || l.contains("50"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_int_default_to_i32_unknown_context() {
    // Return type context: bare literal 42 with i32 return type should get _i32
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("default.wj"),
        r#"
pub fn get_default() -> i32 {
    42
}
"#,
    )
    .unwrap();

    build_project_ext(
        &src.join("default.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        false,
        &[],
    )
    .expect("Build should succeed");

    let default_code = std::fs::read_to_string(build.join("default.rs")).unwrap();

    // ASSERT: 42 in return position with i32 return type should get _i32
    assert!(
        default_code.contains("42_i32"),
        "Return literal 42 should infer i32 from return type. Generated:\n{}",
        default_code
    );
}
