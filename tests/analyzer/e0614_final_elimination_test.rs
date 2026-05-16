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

//! TDD Test: E0614 Final 7 Elimination (Phase 10)
//!
//! Fixes for remaining E0614 "cannot be dereferenced" errors:
//! - Pattern A: Match pattern vars (item_id, delta, points) → *(var).clone() was wrong
//! - Pattern B: Iterator vars (entity) when type is Copy → *entity was wrong
//!
//! Fix 1: expression_is_reference returns false for Identifier when local_var_types
//!        says Copy type (match pattern vars get owned u32/i32 from infer_match_bound_types)
//! Fix 2: When adding * for reference coercion, skip if we'll add .clone() for same arg
//!        (.clone() returns owned - never deref it)

#[path = "../common/test_utils.rs"]
mod test_utils;

// === Pattern A: Match pattern vars (item_id, delta, points) - no *(var).clone() ===

#[test]
fn test_match_pattern_u32_no_deref_clone() {
    // dialogue/system: GiveItem(item_id) => state.give_item(item_id)
    // Should generate: give_item(item_id) NOT *(item_id).clone()
    let source = r#"
pub fn give_item(id: u32) {
}

pub enum Consequence {
    GiveItem(u32),
}

impl Consequence {
    pub fn apply(self) {
        match self {
            Consequence::GiveItem(item_id) => {
                give_item(item_id)
            },
        }
    }
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(item_id).clone()"),
        "Should NOT add *(item_id).clone() for match pattern vars. Generated:\n{}",
        rs
    );
}

#[test]
fn test_match_pattern_i32_no_deref_clone() {
    // dialogue/system: AddHonor(points) => state.add_honor(points)
    let source = r#"
pub fn add_honor(points: i32) {
}

pub enum Consequence {
    AddHonor(i32),
}

impl Consequence {
    pub fn apply(self) {
        match self {
            Consequence::AddHonor(points) => {
                add_honor(points)
            },
        }
    }
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(points).clone()"),
        "Should NOT add *(points).clone() for match pattern vars. Generated:\n{}",
        rs
    );
}

#[test]
fn test_match_pattern_has_item_u32() {
    // dialogue/system: HasItem(item_id) => state.has_item(item_id)
    let source = r#"
pub fn has_item(id: u32) -> bool {
    true
}

pub enum Condition {
    HasItem(u32),
}

impl Condition {
    pub fn is_met(self) -> bool {
        match self {
            Condition::HasItem(item_id) => {
                has_item(item_id)
            },
        }
    }
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(item_id).clone()"),
        "Should NOT add *(item_id).clone() for match pattern vars. Generated:\n{}",
        rs
    );
}
