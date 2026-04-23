use windjammer::analyzer::{FunctionSignature, OwnershipMode};
use windjammer::parser::ast::Type;
/// TDD: Static method call integer argument incorrectly cast to f32
///
/// Bug: When calling `TypeA::new(pos, 30)` where second param is `i32`,
/// the codegen incorrectly generates `30 as f32` if there exists another
/// type with a `new` constructor whose second param IS `f32`.
///
/// Root cause: Multiple types with the same name in different modules
/// register under the same key ("TypeName::new") in the signature registry.
/// When a consumer file doesn't define the type locally, the global registry
/// may return the WRONG type's signature due to namespace collision.
use windjammer::*;

fn compile_to_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let mut generator = codegen::CodeGenerator::new(signatures, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

fn compile_to_rust_with_global_sigs(
    source: &str,
    global_sigs: &analyzer::SignatureRegistry,
) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    let mut analyzer_instance = analyzer::Analyzer::new();
    let (analyzed, local_sigs, _trait_methods) = analyzer_instance
        .analyze_program(&program)
        .expect("Failed to analyze");

    let mut per_file_registry = global_sigs.clone();
    per_file_registry.merge(&local_sigs);

    let mut generator = codegen::CodeGenerator::new(per_file_registry, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

#[test]
fn test_static_call_i32_param_not_cast_to_f32() {
    let source = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}

struct Emitter {
    pos: Vec3,
    max_particles: i32,
    rate: f32,
}

impl Emitter {
    fn new(pos: Vec3, max_particles: i32) -> Emitter {
        Emitter { pos: pos, max_particles: max_particles, rate: 10.0 }
    }
}

fn create_emitter(pos: Vec3) -> Emitter {
    let mut emitter = Emitter::new(pos, 30)
    emitter.rate = 500.0
    emitter
}
"#;
    let rust_code = compile_to_rust(source);

    assert!(
        !rust_code.contains("30 as f32") && !rust_code.contains("30_i32 as f32"),
        "Integer 30 passed to i32 param should NOT be cast to f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_static_call_f32_param_still_casts_int() {
    let source = r#"
struct Config {
    value: f32,
}

impl Config {
    fn new(value: f32) -> Config {
        Config { value: value }
    }
}

fn make_config() -> Config {
    Config::new(42)
}
"#;
    let rust_code = compile_to_rust(source);

    assert!(
        rust_code.contains("42 as f32")
            || rust_code.contains("42_f32")
            || rust_code.contains("42.0"),
        "Integer 42 passed to f32 param SHOULD be cast to f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_multiple_types_with_new_correct_dispatch() {
    let source = r#"
struct TypeA {
    count: i32,
}

impl TypeA {
    fn new(count: i32) -> TypeA {
        TypeA { count: count }
    }
}

struct TypeB {
    value: f32,
}

impl TypeB {
    fn new(value: f32) -> TypeB {
        TypeB { value: value }
    }
}

fn test() {
    let a = TypeA::new(10)
    let b = TypeB::new(20)
}
"#;
    let rust_code = compile_to_rust(source);

    let lines: Vec<&str> = rust_code.lines().collect();
    let mut _found_a_correct = false;
    let mut _found_b_correct = false;

    for line in &lines {
        if line.contains("TypeA::new") {
            assert!(
                !line.contains("10 as f32") && !line.contains("10_i32 as f32"),
                "TypeA::new(10) should NOT cast to f32 (param is i32).\nLine: {}",
                line
            );
            _found_a_correct = true;
        }
        if line.contains("TypeB::new") {
            assert!(
                line.contains("20 as f32") || line.contains("20_f32") || line.contains("20.0"),
                "TypeB::new(20) SHOULD cast to f32 (param is f32).\nLine: {}",
                line
            );
            _found_b_correct = true;
        }
    }
}

/// TDD: Signature collision from different modules should not cause incorrect auto-cast.
///
/// Bug scenario (multi-file):
/// - Module A defines ParticleEmitter::new(f32, f32)
/// - Module B defines ParticleEmitter::new(Vec3, i32)
/// - Consumer file imports from Module B, calls ParticleEmitter::new(pos, 30)
/// - If Module A's signature wins in global registry, codegen incorrectly casts 30→f32
///
/// Fix: When a signature key has been registered with conflicting param types
/// (collision detected), the auto-cast should be skipped for safety.
#[test]
fn test_signature_collision_no_incorrect_cast() {
    // Source defines Emitter with i32 second param
    let source = r#"
struct Vec3 { x: f32, y: f32, z: f32 }

struct Emitter {
    pos: Vec3,
    max_particles: i32,
}

impl Emitter {
    fn new(pos: Vec3, max_particles: i32) -> Emitter {
        Emitter { pos: pos, max_particles: max_particles }
    }
}

fn create(pos: Vec3) -> Emitter {
    Emitter::new(pos, 30)
}
"#;
    // Pre-populate global registry with a WRONG signature from a different module.
    // In multi-file mode, particles/emitter.wj's Emitter::new(f32, f32)
    // would be registered first, then overwritten by effects/particle_emitter.wj's
    // Emitter::new(Vec3, i32). But compilation order may not guarantee this.
    let mut global_sigs = analyzer::SignatureRegistry::new();
    global_sigs.add_function(
        "Emitter::new".to_string(),
        FunctionSignature {
            name: "new".to_string(),
            param_types: vec![
                Type::Custom("f32".to_string()),
                Type::Custom("f32".to_string()),
            ],
            param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Owned],
            return_type: Some(Type::Custom("Emitter".to_string())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    let rust_code = compile_to_rust_with_global_sigs(source, &global_sigs);

    // Local signature (Vec3, i32) should take priority over global (f32, f32)
    assert!(
        !rust_code.contains("30 as f32") && !rust_code.contains("30_i32 as f32"),
        "Integer 30 passed to i32 param should NOT be cast to f32, even when global registry has wrong signature.\nGenerated:\n{}",
        rust_code
    );
}

/// TDD: Consumer file without local definition should detect signature collision
/// and avoid incorrect auto-cast.
///
/// This simulates the EXACT multi-file bug: consumer file doesn't define the type,
/// relies on global registry, but global has the WRONG signature winning.
#[test]
fn test_signature_collision_consumer_wrong_sig_wins() {
    // Consumer file that calls Emitter::new without defining it
    let source = r#"
fn create() {
    let e = Emitter::new(1.0, 30)
}
"#;
    let mut global_sigs = analyzer::SignatureRegistry::new();

    // First registration: correct signature from effects/particle_emitter.wj
    global_sigs.add_function(
        "Emitter::new".to_string(),
        FunctionSignature {
            name: "new".to_string(),
            param_types: vec![
                Type::Custom("Vec3".to_string()),
                Type::Custom("i32".to_string()),
            ],
            param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Owned],
            return_type: Some(Type::Custom("Emitter".to_string())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    // Second registration: WRONG signature from particles/emitter.wj (this one WINS)
    // This OVERWRITES the first. The collision should be detected.
    global_sigs.add_function(
        "Emitter::new".to_string(),
        FunctionSignature {
            name: "new".to_string(),
            param_types: vec![
                Type::Custom("f32".to_string()),
                Type::Custom("f32".to_string()),
            ],
            param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Owned],
            return_type: Some(Type::Custom("Emitter".to_string())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    let rust_code = compile_to_rust_with_global_sigs(source, &global_sigs);

    // 30 should NOT be cast to f32 because there's a collision on "Emitter::new"
    // Even though the current winning signature says (f32, f32), the collision flag
    // should prevent the auto-cast since we can't be sure which signature is right.
    assert!(
        !rust_code.contains("30 as f32"),
        "Integer 30 should NOT be cast to f32 when signature has collision.\nGenerated:\n{}",
        rust_code
    );
}

/// TDD: Collision detection should track which keys have conflicting signatures.
#[test]
fn test_signature_registry_collision_detection() {
    let mut registry = analyzer::SignatureRegistry::new();

    // First registration
    registry.add_function(
        "Foo::new".to_string(),
        FunctionSignature {
            name: "new".to_string(),
            param_types: vec![Type::Custom("f32".to_string())],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: Some(Type::Custom("Foo".to_string())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    assert!(
        !registry.has_collision("Foo::new"),
        "No collision after first registration"
    );

    // Second registration with SAME param types → not a collision
    registry.add_function(
        "Foo::new".to_string(),
        FunctionSignature {
            name: "new".to_string(),
            param_types: vec![Type::Custom("f32".to_string())],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: Some(Type::Custom("Foo".to_string())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    assert!(
        !registry.has_collision("Foo::new"),
        "Same signature should not be a collision"
    );

    // Third registration with DIFFERENT param types → collision!
    registry.add_function(
        "Foo::new".to_string(),
        FunctionSignature {
            name: "new".to_string(),
            param_types: vec![Type::Custom("i32".to_string())],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: Some(Type::Custom("Foo".to_string())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    assert!(
        registry.has_collision("Foo::new"),
        "Different param types should be a collision"
    );
}

/// TDD: Collision flag should survive merge operations.
#[test]
fn test_collision_survives_merge() {
    let mut reg1 = analyzer::SignatureRegistry::new();
    reg1.add_function(
        "Bar::new".to_string(),
        FunctionSignature {
            name: "new".to_string(),
            param_types: vec![Type::Custom("f32".to_string())],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    let mut reg2 = analyzer::SignatureRegistry::new();
    reg2.add_function(
        "Bar::new".to_string(),
        FunctionSignature {
            name: "new".to_string(),
            param_types: vec![Type::Custom("i32".to_string())],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    reg1.merge(&reg2);

    assert!(
        reg1.has_collision("Bar::new"),
        "Collision should be detected during merge"
    );
}
