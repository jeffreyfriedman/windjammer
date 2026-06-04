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

use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let src_path = dir.path().join("test.wj");
    let out_dir = dir.path().join("out");
    std::fs::write(&src_path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", src_path.to_str().unwrap(), "--no-cargo", "-o"])
        .arg(out_dir.to_str().unwrap())
        .output()
        .unwrap();
    if !output.status.success() {
        panic!(
            "wj build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let rs_path = out_dir.join("test.rs");
    std::fs::read_to_string(&rs_path).unwrap_or_default()
}

/// When a method has implicit self (not listed as a parameter) and the analyzer
/// infers &self, for-loops over self.field should iterate by reference (&self.field).
/// Bug: the fallback path in function_generation adds &self but doesn't register
/// "self" in inferred_borrowed_params, so for-loop borrow detection fails.
#[test]
fn test_implicit_self_for_loop_borrows_field() {
    let source = r#"
struct Condition {
    met: bool
}

struct Rule {
    conditions: Vec<Condition>
    name: string
}

impl Rule {
    pub fn all_met() -> bool {
        for condition in self.conditions {
            if !condition.met {
                return false
            }
        }
        true
    }
}
"#;
    let output = compile_wj(source);
    // The for-loop should borrow self.conditions since self is &self
    assert!(
        output.contains("for condition in &self.conditions")
            || output.contains("for condition in & self.conditions"),
        "For-loop on self.field with implicit &self should borrow. Got:\n{}",
        output
    );
}

/// When a method has implicit self and modifies fields (so &mut self),
/// for-loops over self.field should iterate by reference too.
#[test]
fn test_implicit_mut_self_for_loop_borrows_field() {
    let source = r#"
struct Item {
    value: i32
}

struct Inventory {
    items: Vec<Item>
    total: i32
}

impl Inventory {
    pub fn recalculate() {
        self.total = 0
        for item in self.items {
            self.total = self.total + item.value
        }
    }
}
"#;
    let output = compile_wj(source);
    // The for-loop should borrow self.items since self is &mut self
    assert!(
        output.contains("for item in &self.items")
            || output.contains("for item in &mut self.items"),
        "For-loop on self.field with implicit &mut self should borrow. Got:\n{}",
        output
    );
}

/// When matching on self.field where self is borrowed, the match scrutinee
/// should be borrowed to prevent moving out of the reference.
#[test]
fn test_match_self_field_borrows_scrutinee() {
    let source = r#"
enum Cost {
    Gold(i32)
    Item(string, i32)
    Free
}

struct Offer {
    cost: Cost
    name: string
}

impl Offer {
    pub fn describe(self) -> string {
        match self.cost {
            Cost::Gold(amount) => format!("Costs {} gold", amount),
            Cost::Item(item_id, qty) => format!("Costs {} x{}", item_id, qty),
            Cost::Free => "Free".to_string()
        }
    }
}
"#;
    let output = compile_wj(source);
    // match should borrow self.cost or clone it to prevent moving
    assert!(
        output.contains("match &self.cost") || output.contains("match self.cost.clone()"),
        "Match on self.field with &self should borrow or clone scrutinee. Got:\n{}",
        output
    );
}

/// When assigning self.field (Option type) to a local variable where self is &mut self,
/// the generated Rust should use .take() instead of a direct move.
#[test]
fn test_option_field_take_in_mut_self() {
    let source = r#"
struct Weapon {
    name: string
    damage: i32
}

struct Character {
    weapon: Option<Weapon>
}

impl Character {
    pub fn unequip(self) -> Option<Weapon> {
        let prev = self.weapon
        self.weapon = None
        prev
    }
}
"#;
    let output = compile_wj(source);
    assert!(
        output.contains("self.weapon.take()")
            || output.contains("std::mem::take(&mut self.weapon)")
            || output.contains("std::mem::replace(&mut self.weapon"),
        "Option field move-out from &mut self should use .take(). Got:\n{}",
        output
    );
    // The redundant `self.weapon = None` must NOT appear — it's folded into .take()
    assert!(
        !output.contains("self.weapon = None"),
        "Redundant assignment after .take() should be eliminated. Got:\n{}",
        output
    );
}

/// When assigning self.field (Option type) and then immediately overwriting it,
/// the "swap" pattern should use .replace() — the idiomatic Rust for Option swap.
/// `let prev = self.weapon; self.weapon = Some(w)` → `let prev = self.weapon.replace(w)`
#[test]
fn test_option_field_equip_swap_pattern() {
    let source = r#"
struct Weapon {
    name: string
    damage: i32
}

struct Character {
    weapon: Option<Weapon>
}

impl Character {
    pub fn equip(self, w: Weapon) -> Option<Weapon> {
        let prev = self.weapon
        self.weapon = Some(w)
        prev
    }
}
"#;
    let output = compile_wj(source);
    assert!(
        output.contains("self.weapon.replace(w)")
            || output.contains("self.weapon.take()")
            || output.contains("std::mem::replace(&mut self.weapon"),
        "Option field swap should use .replace(), .take(), or mem::replace. Got:\n{}",
        output
    );
    // The redundant `self.weapon = Some(w)` must NOT appear — it's folded into .replace()
    assert!(
        !output.contains("self.weapon = Some(w)"),
        "Redundant assignment after .replace() should be eliminated. Got:\n{}",
        output
    );
}

/// When an enum method uses `match self` but arms only compare/ignore bound
/// values (no consuming moves), the compiler should infer &self and use match
/// ergonomics. No auto-clone is needed at call sites.
/// Reproduces the breach-protocol dialog.wj pattern: is_available(&self) calls
/// condition.evaluate(state) inside a for-loop.
#[test]
fn test_enum_match_self_non_consuming_arms_infers_borrowed() {
    let source = r#"
enum Condition {
    HasGold(i32)
    Custom(string)
}

struct GameState {
    gold: i32
}

impl Condition {
    pub fn evaluate(self, state: GameState) -> bool {
        match self {
            Condition::HasGold(amount) => state.gold >= amount,
            Condition::Custom(id) => false,
        }
    }
}

struct Rule {
    conditions: Vec<Condition>
}

impl Rule {
    pub fn is_available(state: GameState) -> bool {
        if self.conditions.is_empty() {
            return true
        }
        for condition in self.conditions {
            if !condition.evaluate(state) {
                return false
            }
        }
        true
    }
}
"#;
    let output = compile_wj(source);
    // evaluate's match arms only compare bound values (non-consuming) so infer &self
    assert!(
        output.contains("fn evaluate(&self"),
        "Enum match with non-consuming arms should infer &self. Got:\n{}",
        output
    );
    // Rule::is_available should be &self too
    assert!(
        output.contains("fn is_available(&self"),
        "is_available should be &self. Got:\n{}",
        output
    );
    // No auto-clone needed: evaluate takes &self, loop variable is &Condition
    assert!(
        !output.contains("condition.clone().evaluate"),
        "No auto-clone needed when evaluate takes &self. Got:\n{}",
        output
    );
}

/// When an enum method's match arms MOVE a non-Copy bound value out (e.g.,
/// returning it), the compiler must infer `self` (owned). Call sites on
/// borrowed loop variables then need auto-clone.
#[test]
fn test_enum_match_self_consuming_arms_needs_auto_clone() {
    let source = r#"
enum Payload {
    Text(string)
    Empty
}

impl Payload {
    pub fn extract(self) -> string {
        match self {
            Payload::Text(value) => value,
            Payload::Empty => "",
        }
    }
}

struct MessageQueue {
    payloads: Vec<Payload>
}

impl MessageQueue {
    pub fn extract_all() -> Vec<string> {
        let mut result = Vec::new()
        for payload in self.payloads {
            result.push(payload.extract())
        }
        result
    }
}
"#;
    let output = compile_wj(source);
    // extract moves `value` (String) out of the enum — must be self (owned)
    assert!(
        output.contains("fn extract(self") || output.contains("fn extract(mut self"),
        "Enum match that moves non-Copy bound value should infer owned self. Got:\n{}",
        output
    );
}
