#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]
/// TDD Test: self.field.user_method() must infer &mut self when user_method mutates
///
/// Bug: When a method body only calls `self.state.tick(dt)` (no direct `self.field = ...`),
/// the analyzer fails to infer &mut self because:
/// 1. `expression_is_self_field_mutating_method_call` returns false for unknown user methods
/// 2. The registry may not yet have the callee's signature during early passes
///
/// This causes E0596 in generated Rust: "cannot borrow `*self` as mutable"
///
/// Fix: Conservative default for unknown self.field methods -- assume mutation.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_self_field_user_method_infers_mut_single_file() {
    let source = r#"
pub struct SubSystem {
    pub value: i32,
}

impl SubSystem {
    pub fn tick(self, dt: f32) {
        self.value = self.value + 1
    }

    pub fn get_value(self) -> i32 {
        return self.value
    }
}

pub struct GameState {
    pub sub: SubSystem,
}

impl GameState {
    pub fn update(self, dt: f32) {
        self.sub.tick(dt)
    }
}
"#;

    let generated = test_utils::compile_single_result(source)
        .expect("Windjammer compilation should succeed");

    assert!(
        generated.contains("fn update(&mut self"),
        "update() calls self.sub.tick() which mutates, so must infer &mut self.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_self_field_unknown_user_method_conservative_default() {
    let source = r#"
pub struct CombatState {
    pub damage: f32,
}

impl CombatState {
    pub fn process_hit(self, amount: f32) {
        self.damage = self.damage + amount
    }
}

pub struct HudState {
    pub dirty: bool,
}

impl HudState {
    pub fn refresh(self) {
        self.dirty = false
    }
}

pub struct Game {
    pub combat: CombatState,
    pub hud: HudState,
}

impl Game {
    pub fn update(self, dt: f32) {
        self.combat.process_hit(10.0)
        self.hud.refresh()
    }
}
"#;

    let generated = test_utils::compile_single_result(source)
        .expect("Windjammer compilation should succeed");

    assert!(
        generated.contains("fn update(&mut self"),
        "update() calls self.combat.process_hit() and self.hud.refresh() which both mutate,\
         so must infer &mut self.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_self_field_readonly_method_stays_borrowed() {
    let source = r#"
pub struct Stats {
    pub hp: i32,
}

impl Stats {
    pub fn get_hp(self) -> i32 {
        return self.hp
    }
}

pub struct Player {
    pub stats: Stats,
}

impl Player {
    pub fn is_alive(self) -> bool {
        return self.stats.get_hp() > 0
    }
}
"#;

    let generated = test_utils::compile_single_result(source)
        .expect("Windjammer compilation should succeed");

    assert!(
        generated.contains("fn is_alive(&self") || generated.contains("fn is_alive(self"),
        "is_alive() only reads -- should be &self or self (Copy types get self by value).\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("fn is_alive(&mut self"),
        "is_alive() only reads -- must NOT be &mut self.\nGenerated:\n{}",
        generated
    );
}
