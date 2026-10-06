#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! WDB-344: Copy `f32` must not emit `.clone()` on locals / match payloads.
//!
//! Product tip game-core `physics/collision2d.rs`:
//!   `check_aabb_vs_aabb(…, *w1.clone(), *h1.clone(), …)` from `match &a.collider`
//!   Copy `f32` field bindings — prefer `*w1` / bare Copy (never `*w1.clone()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub fn mk_normal(normal_x: f32, normal_y: f32) -> Vec2 {
    Vec2 {
        x: normal_x,
        y: normal_y,
    }
}
"#;

/// Demoted `&Body` + nested match on collider fields reused across exclusive arms
/// (product `check_collision` shape that emitted `*w1.clone()`).
const MATCH_SRC: &str = r#"
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub enum Collider2D {
    Box { width: f32, height: f32 },
    Circle { radius: f32 },
}

pub struct Body {
    pub position: Vec2,
    pub collider: Collider2D,
    pub collision_layer: u32,
    pub collision_mask: u32,
}

fn check_aabb_vs_aabb(a: Vec2, w1: f32, h1: f32, b: Vec2, w2: f32, h2: f32) -> bool {
    true
}

fn check_aabb_vs_circle(a: Vec2, w1: f32, h1: f32, b: Vec2, r: f32) -> bool {
    true
}

pub fn check_collision(a: Body, b: Body) -> bool {
    if (a.collision_layer & b.collision_mask) == 0 {
        return false
    }
    match a.collider {
        Collider2D::Box { width: w1, height: h1 } => {
            match b.collider {
                Collider2D::Box { width: w2, height: h2 } => {
                    check_aabb_vs_aabb(a.position, w1, h1, b.position, w2, h2)
                }
                Collider2D::Circle { radius: r } => {
                    check_aabb_vs_circle(a.position, w1, h1, b.position, r)
                }
            }
        }
        Collider2D::Circle { radius: r1 } => {
            match b.collider {
                Collider2D::Box { width: w, height: h } => {
                    check_aabb_vs_circle(b.position, w, h, a.position, r1)
                }
                Collider2D::Circle { radius: r2 } => {
                    true
                }
            }
        }
    }
}
"#;

fn has_star_copy_clone(rs: &str) -> bool {
    rs.lines().any(|line| {
        let t = line.trim_start();
        !t.starts_with("//")
            && (line.contains("*.clone()")
                || line.contains("*w1.clone()")
                || line.contains("*h1.clone()")
                || line.contains("*w2.clone()")
                || line.contains("*h2.clone()")
                || line.contains("normal_x.clone()")
                || line.contains("normal_y.clone()"))
    })
}

#[test]
fn wdb344_module_file_copy_f32_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-344 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-344 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-344 RED: Copy f32 emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-344 cargo-check");
}

#[test]
fn wdb344_module_file_match_ref_copy_f32_must_not_star_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", MATCH_SRC);
    let map = test.compile().expect("WDB-344 match compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-344 match MultiFile lib.rs:\n{rs}");
    assert!(
        !has_star_copy_clone(rs),
        "WDB-344 RED: match &Copy f32 emitted *binding.clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-344 match cargo-check");
}

#[test]
fn wdb344_tip_out_game_core_collision2d_must_not_clone_f32() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("collision2d.rs"),
        tip.join("physics/collision2d.rs"),
        game.join("gen/physics/collision2d.rs"),
        game.join("physics/collision2d.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("collision2d");
        if has_star_copy_clone(&text) {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-344: game-core/tip collision2d missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-344 RED: tip/product clones Copy f32 in:\n  {}",
        bad_paths.join("\n  ")
    );
}
