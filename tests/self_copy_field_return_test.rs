use std::collections::HashMap;
/// TDD: Methods returning Copy-type fields should infer `&self`, not consuming `self`
///
/// Bug: When a method body is just `self.field` and the field type is a Copy struct
/// (like Vec3 from another crate), the analyzer treats it as moving a non-Copy field,
/// forcing `self` to be owned/consuming. This forces callers to `.clone()` the parent
/// struct unnecessarily.
///
/// Fix: The analyzer must recognize common Copy structs (Vec3, Vec2, etc.) and also
/// support cross-crate Copy detection via `.wj.meta` metadata.
///
/// Windjammer principle: "Compiler does the hard work, not the developer."
use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::metadata::infer_copy_from_metadata_structs_pub;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn compile_to_rust(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();

    let mut analyzer = Analyzer::new();
    let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("Analysis failed");

    let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    codegen.generate_program(&program, &analyzed)
}

fn compile_with_copy_structs(source: &str, copy_structs: &[&str]) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();

    let mut analyzer = Analyzer::new();
    for name in copy_structs {
        analyzer.register_copy_struct(name);
    }
    let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("Analysis failed");

    let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    codegen.generate_program(&program, &analyzed)
}

#[test]
fn test_copy_field_return_infers_borrowed_self() {
    // Vec3 with all-Copy fields should auto-detect as Copy,
    // so returning self.position should use &self
    let source = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

struct Player {
    position: Vec3,
    name: string,
}

impl Player {
    pub fn get_position(self) -> Vec3 {
        self.position
    }
}
"#;

    let code = compile_to_rust(source);

    assert!(
        code.contains("fn get_position(&self)"),
        "get_position should use &self since Vec3 is Copy (all-Copy fields). Got:\n{}",
        code
    );
}

#[test]
fn test_non_copy_field_return_with_clone_infers_borrowed_self() {
    // When method body is `self.name.clone()`, self should be &self
    // because .clone() on a field through &self creates a new owned value
    let source = r#"
struct Player {
    name: string,
    score: i32,
}

impl Player {
    pub fn get_name(self) -> string {
        self.name.clone()
    }
}
"#;

    let code = compile_to_rust(source);

    assert!(
        code.contains("fn get_name(&self)"),
        "get_name with .clone() should use &self. Got:\n{}",
        code
    );
}

#[test]
fn test_external_copy_struct_via_register() {
    // When Vec3 is from another crate and registered as Copy,
    // methods returning self.field of that type should use &self
    let source = r#"
struct Container {
    position: Vec3,
}

impl Container {
    pub fn get_position(self) -> Vec3 {
        self.position
    }
}
"#;

    let code = compile_with_copy_structs(source, &["Vec3"]);

    assert!(
        code.contains("fn get_position(&self)"),
        "get_position should use &self when Vec3 is registered as Copy. Got:\n{}",
        code
    );
}

#[test]
fn test_caller_no_clone_needed_for_borrowed_getter() {
    // When calling a &self getter on a field accessed through &mut self,
    // no .clone() on the parent struct should be needed
    let source = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

struct LevelLoader {
    player_spawn: Vec3,
    loaded: bool,
}

impl LevelLoader {
    pub fn get_player_spawn(self) -> Vec3 {
        self.player_spawn
    }
}

struct Game {
    level_loader: LevelLoader,
    player_x: f32,
}

impl Game {
    pub fn init(self) {
        let pos = self.level_loader.get_player_spawn()
        self.player_x = pos.x
    }
}
"#;

    let code = compile_to_rust(source);

    assert!(
        code.contains("fn get_player_spawn(&self)"),
        "get_player_spawn should use &self since Vec3 is Copy. Got:\n{}",
        code
    );

    assert!(
        !code.contains("level_loader.clone().get_player_spawn"),
        "Should NOT need .clone() on level_loader to call &self getter. Got:\n{}",
        code
    );
}

#[test]
fn test_infer_copy_from_metadata_struct_fields() {
    // Simulate what happens when loading a .wj.meta file with struct field definitions.
    // Vec3 has all f32 fields -> should be auto-detected as Copy from metadata.
    // TDD FIX: Updated to use Vec<Vec<String>> for conservative Copy detection
    let mut struct_fields: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    struct_fields.insert(
        "Vec3".to_string(),
        vec![vec![
            "Custom(\"f32\")".to_string(),
            "Custom(\"f32\")".to_string(),
            "Custom(\"f32\")".to_string(),
        ]],
    );
    struct_fields.insert(
        "Vec2".to_string(),
        vec![vec![
            "Custom(\"f32\")".to_string(),
            "Custom(\"f32\")".to_string(),
        ]],
    );
    struct_fields.insert(
        "Player".to_string(),
        vec![vec![
            "Custom(\"Vec3\")".to_string(), // Vec3 should be Copy
            "String".to_string(),           // String is NOT Copy
        ]],
    );
    struct_fields.insert(
        "Color".to_string(),
        vec![vec![
            "Custom(\"f32\")".to_string(),
            "Custom(\"f32\")".to_string(),
            "Custom(\"f32\")".to_string(),
            "Custom(\"f32\")".to_string(),
        ]],
    );
    // Transitive Copy: AABB has all Vec3 fields -> should be Copy if Vec3 is Copy
    struct_fields.insert(
        "AABB".to_string(),
        vec![vec![
            "Custom(\"Vec3\")".to_string(),
            "Custom(\"Vec3\")".to_string(),
        ]],
    );

    let mut copy_structs = Vec::new();
    infer_copy_from_metadata_structs_pub(&struct_fields, &mut copy_structs);

    assert!(
        copy_structs.contains(&"Vec3".to_string()),
        "Vec3 (all f32 fields) should be Copy"
    );
    assert!(
        copy_structs.contains(&"Vec2".to_string()),
        "Vec2 (all f32 fields) should be Copy"
    );
    assert!(
        copy_structs.contains(&"Color".to_string()),
        "Color (all f32 fields) should be Copy"
    );
    assert!(
        copy_structs.contains(&"AABB".to_string()),
        "AABB (all Vec3 fields) should be Copy via transitive detection"
    );
    assert!(
        !copy_structs.contains(&"Player".to_string()),
        "Player (has String field) should NOT be Copy"
    );
}

#[test]
fn test_metadata_copy_structs_integrate_with_analyzer() {
    // Simulate the full flow: metadata has struct with all-Copy fields,
    // analyzer uses that to correctly infer &self for getter methods.
    // TDD FIX: Updated to use Vec<Vec<String>> for conservative Copy detection
    let mut struct_fields: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    struct_fields.insert(
        "Vec3".to_string(),
        vec![vec![
            "Custom(\"f32\")".to_string(),
            "Custom(\"f32\")".to_string(),
            "Custom(\"f32\")".to_string(),
        ]],
    );

    let mut copy_structs = Vec::new();
    infer_copy_from_metadata_structs_pub(&struct_fields, &mut copy_structs);

    // Now use these Copy structs with the analyzer
    let source = r#"
struct Container {
    position: Vec3,
}

impl Container {
    pub fn get_position(self) -> Vec3 {
        self.position
    }
}
"#;

    let code = compile_with_copy_structs(
        source,
        &copy_structs.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );

    assert!(
        code.contains("fn get_position(&self)"),
        "get_position should use &self when Vec3 is detected as Copy from metadata fields. Got:\n{}",
        code
    );
}
