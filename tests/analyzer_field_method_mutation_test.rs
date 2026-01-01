// TDD Test: Methods that call mutating methods on fields need &mut self
// THE WINDJAMMER WAY: Compiler infers mutability from usage!

use windjammer::analyzer::Analyzer;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};

fn parse_code(code: &str) -> Program<'static> {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    parser.parse().unwrap()
}

#[test]
fn test_method_calling_field_mutating_method_needs_mut_self() {
    let code = r#"
struct Allocator {
    next_id: i32,
}

impl Allocator {
    pub fn allocate(self) -> i32 {
        self.next_id += 1
        return self.next_id
    }
}

struct World {
    allocator: Allocator,
}

impl World {
    pub fn spawn(self) -> i32 {
        return self.allocator.allocate()
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).unwrap();

    // Find the spawn function
    let spawn_fn = analyzed
        .iter()
        .find(|f| f.decl.name == "spawn")
        .expect("spawn function not found");

    // ASSERT: spawn should have &mut self because it calls allocate() on self.allocator,
    // and allocate() mutates self (it does self.next_id += 1)
    let self_ownership = spawn_fn
        .inferred_ownership
        .get("self")
        .expect("self parameter should exist");
    assert_eq!(
        *self_ownership,
        windjammer::analyzer::OwnershipMode::MutBorrowed,
        "spawn() should infer &mut self because it calls self.allocator.allocate() which mutates!"
    );
}

#[test]
fn test_method_calling_field_nonmutating_method_uses_ref_self() {
    let code = r#"
struct Config {
    max_value: i32,
}

impl Config {
    pub fn get_max(self) -> i32 {
        return self.max_value
    }
}

struct World {
    config: Config,
}

impl World {
    pub fn max_entities(self) -> i32 {
        return self.config.get_max()
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).unwrap();

    // Find the max_entities function
    let max_entities_fn = analyzed
        .iter()
        .find(|f| f.decl.name == "max_entities")
        .expect("max_entities function not found");

    // ASSERT: max_entities should have &self because get_max() doesn't mutate
    let self_ownership = max_entities_fn
        .inferred_ownership
        .get("self")
        .expect("self parameter should exist");
    assert_eq!(
        *self_ownership,
        windjammer::analyzer::OwnershipMode::Borrowed,
        "max_entities() should infer &self because get_max() doesn't mutate"
    );
}

#[test]
fn test_method_calling_remove_on_field_needs_mut_self() {
    let code = r#"
use std::collections::HashMap

struct World {
    transforms: HashMap<i32, i32>,
}

impl World {
    pub fn remove_transform(self, entity: i32) {
        self.transforms.remove(entity)
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).unwrap();

    // Find the remove_transform function
    let remove_fn = analyzed
        .iter()
        .find(|f| f.decl.name == "remove_transform")
        .expect("remove_transform function not found");

    // ASSERT: remove_transform should have &mut self because it calls remove() on a HashMap
    let self_ownership = remove_fn
        .inferred_ownership
        .get("self")
        .expect("self parameter should exist");
    assert_eq!(
        *self_ownership,
        windjammer::analyzer::OwnershipMode::MutBorrowed,
        "remove_transform() should infer &mut self because HashMap::remove mutates!"
    );
}

#[test]
fn test_method_calling_get_mut_on_field_needs_mut_self() {
    let code = r#"
use std::collections::HashMap

struct World {
    dirty_flags: HashMap<i32, bool>,
}

impl World {
    pub fn mark_dirty(self, entity: i32) {
        let flag = self.dirty_flags.get_mut(entity)
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).unwrap();

    // Find the mark_dirty function
    let mark_dirty_fn = analyzed
        .iter()
        .find(|f| f.decl.name == "mark_dirty")
        .expect("mark_dirty function not found");

    // ASSERT: mark_dirty should have &mut self because it calls get_mut()
    let self_ownership = mark_dirty_fn
        .inferred_ownership
        .get("self")
        .expect("self parameter should exist");
    assert_eq!(
        *self_ownership,
        windjammer::analyzer::OwnershipMode::MutBorrowed,
        "mark_dirty() should infer &mut self because get_mut requires mutable access!"
    );
}

#[test]
fn test_if_let_with_get_mut_needs_mut_self() {
    let code = r#"
use std::collections::HashMap

struct World {
    dirty_flags: HashMap<i32, bool>,
}

impl World {
    pub fn mark_dirty(self, entity: i32) {
        if let Some(flag) = self.dirty_flags.get_mut(entity) {
            *flag = true
        }
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).unwrap();

    // Find the mark_dirty function
    let mark_dirty_fn = analyzed
        .iter()
        .find(|f| f.decl.name == "mark_dirty")
        .expect("mark_dirty function not found");

    // ASSERT: mark_dirty should have &mut self because if-let condition calls get_mut()
    let self_ownership = mark_dirty_fn
        .inferred_ownership
        .get("self")
        .expect("self parameter should exist");
    assert_eq!(
        *self_ownership,
        windjammer::analyzer::OwnershipMode::MutBorrowed,
        "mark_dirty() should infer &mut self because if-let condition calls get_mut!"
    );
}

#[test]
fn test_field_update_method_needs_mut_self() {
    let code = r#"
struct AnimatedSprite {
    frame: i32,
}

impl AnimatedSprite {
    pub fn update(self, delta: f32) {
        self.frame = self.frame + 1
    }
    
    pub fn tick(self, delta: f32) {
        self.update(delta)
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).unwrap();

    // Find the tick function
    let tick_fn = analyzed
        .iter()
        .find(|f| f.decl.name == "tick")
        .expect("tick function not found");

    // ASSERT: tick should have &mut self because it calls self.update() which mutates self
    let self_ownership = tick_fn
        .inferred_ownership
        .get("self")
        .expect("self parameter should exist");
    assert_eq!(
        *self_ownership,
        windjammer::analyzer::OwnershipMode::MutBorrowed,
        "tick() should infer &mut self because it calls self.update() which mutates self!"
    );
}

#[test]
fn test_calling_update_on_field_needs_mut_self() {
    let code = r#"
struct InnerSprite {
    frame: i32,
}

impl InnerSprite {
    pub fn update(self, delta: f32) {
        self.frame = self.frame + 1
    }
}

struct OuterSprite {
    sprite: InnerSprite,
}

impl OuterSprite {
    pub fn tick(self, delta: f32) {
        self.sprite.update(delta)
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).unwrap();

    // Find the tick function
    let tick_fn = analyzed
        .iter()
        .find(|f| f.decl.name == "tick")
        .expect("tick function not found");

    // ASSERT: tick should have &mut self because it calls update() on self.sprite
    let self_ownership = tick_fn
        .inferred_ownership
        .get("self")
        .expect("self parameter should exist");
    assert_eq!(
        *self_ownership,
        windjammer::analyzer::OwnershipMode::MutBorrowed,
        "tick() should infer &mut self because it calls self.sprite.update()!"
    );
}
