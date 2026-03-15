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

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "wj_e0614_final_{}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        id
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

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
    let (rs, compiles) = compile_wj_to_rust(source);
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
    let (rs, compiles) = compile_wj_to_rust(source);
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
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(item_id).clone()"),
        "Should NOT add *(item_id).clone() for match pattern vars. Generated:\n{}",
        rs
    );
}
