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

/// TDD Test: Method call mut propagation - calling mutating methods on fields
///
/// Bug: Calling self.field.mutating_method() doesn't propagate &mut self inference.
/// Method A calls self.field.mutating_method() → method A needs &mut self
/// But analyzer generates &self instead!
///
/// Root Cause: Analyzer checks direct field mutation (self.field = ...)
/// but doesn't properly check method calls on fields that require &mut
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_method_calling_field_mutating_method() {
    // Exact pattern from breach-protocol faction.wj
    let code = r#"
pub struct Faction {
    pub reputation: f32,
}

impl Faction {
    pub fn adjust_reputation(self, amount: f32) {
        self.reputation = self.reputation + amount
    }
}

pub struct FactionManager {
    pub factions: Vec<Faction>,
}

impl FactionManager {
    pub fn adjust_reputation(self, index: usize, amount: f32) {
        self.factions[index].adjust_reputation(amount)
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Calling mutating method on field should compile:\n{}",
        result.err().unwrap_or_default()
    );

    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn adjust_reputation(&mut self, index: usize"),
        "FactionManager::adjust_reputation should have &mut self. Generated:\n{}",
        rust
    );

    // Verify generated Rust compiles
    let verify = test_utils::verify_rust_compiles(&rust);
    assert!(
        verify.is_ok(),
        "Generated Rust should compile:\n{}",
        verify.err().unwrap_or_default()
    );
}

#[test]
fn test_nested_field_method_call() {
    let code = r#"
pub struct Player {
    pub health: f32,
}

impl Player {
    pub fn damage(self, amount: f32) {
        self.health = self.health - amount
    }
}

pub struct Game {
    pub player: Player,
}

impl Game {
    pub fn damage_player(self, amount: f32) {
        self.player.damage(amount)
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Nested field method call should compile:\n{}",
        result.err().unwrap_or_default()
    );

    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn damage_player(&mut self, amount: f32)"),
        "Game::damage_player should have &mut self. Generated:\n{}",
        rust
    );
}

#[test]
fn test_vec_push_on_field() {
    let code = r#"
pub struct EventLog {
    pub events: Vec<string>,
}

impl EventLog {
    pub fn log(self, message: string) {
        self.events.push(message)
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Vec::push on field should compile:\n{}",
        result.err().unwrap_or_default()
    );

    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn log(&mut self,"),
        "EventLog::log should have &mut self. Generated:\n{}",
        rust
    );
}

#[test]
fn test_breach_protocol_faction_pattern() {
    // Exact pattern from breach-protocol: while loop with if, calling adjust_reputation
    let code = r#"
pub enum FactionId {
    A,
    B,
}

impl FactionId {
    pub fn equals(self, other: FactionId) -> bool {
        true
    }
}

pub struct Faction {
    pub id: FactionId,
    pub reputation: f32,
}

impl Faction {
    pub fn adjust_reputation(self, amount: f32) {
        self.reputation = self.reputation + amount
    }
}

pub struct FactionSystem {
    pub factions: Vec<Faction>,
}

impl FactionSystem {
    pub fn adjust_reputation(self, id: FactionId, amount: f32) {
        let mut i = 0
        while i < self.factions.len() {
            if self.factions[i].id.equals(id) {
                self.factions[i].adjust_reputation(amount)
                return
            }
            i = i + 1
        }
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Breach-protocol faction pattern should compile:\n{}",
        result.err().unwrap_or_default()
    );

    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn adjust_reputation(&mut self, id: FactionId, amount: f32)"),
        "FactionSystem::adjust_reputation should have &mut self. Generated:\n{}",
        rust
    );
}
