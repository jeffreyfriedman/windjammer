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

//! TDD: Remaining E0277 from dogfooding (trait bound / comparison / mixed float).
//!
//! Covers:
//! 1. Tuple destructuring from `&vec[i]` → `&String == String` (dialogue get_relationship)
//! 2. `Vec::with_capacity` + `u8` indexing → must not emit `&mask[i]` when element is Copy-in-practice
//! 3. Mixed f32/f64: f64 literal × `as f32` and f32 chain + f64 literal

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn rustc_check(rs: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir for rustc");
    let p = dir.path().join("verify.rs");
    fs::write(&p, rs).unwrap();
    let out = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
            "--edition",
            "2021",
            "-o",
        ])
        .arg(dir.path().join("verify.rmeta"))
        .arg(&p)
        .output()
        .expect("rustc");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (out.status.success(), stderr)
}

#[test]
fn test_tuple_destructure_string_compare_owned_param() {
    let src = r#"
pub struct DialogueState {
    pub relationships: Vec<(string, i32)>,
}

impl DialogueState {
    pub fn get_relationship(self, npc: string) -> i32 {
        for i in 0..self.relationships.len() {
            let (name, score) = self.relationships[i]
            if name == npc {
                return score
            }
        }
        return 0
    }
}
"#;
    let rs = test_utils::compile_single(src);
    let (ok, err) = rustc_check(&rs);
    assert!(ok, "E0277 or other rustc error:\n{}\n\n{}", err, rs);
    // Valid patterns for String/&str comparison:
    // name == npc (Rust's PartialEq<&str> for &String handles this)
    // *name == npc (deref &String to String for PartialEq<&str>)
    // name == &npc (borrow npc for PartialEq<&String>)
    assert!(
        rs.contains("name == npc")
            || rs.contains("== &npc")
            || rs.contains("== & npc")
            || rs.contains("*name == npc")
            || rs.contains("*(name) == npc"),
        "expected valid String comparison; got:\n{}",
        rs
    );
}

#[test]
fn test_vec_with_capacity_u8_index_compare_int() {
    let src = r#"
pub fn fill_mask(width: i32, height: i32) -> i32 {
    let mask_size = (width * height) as usize
    let mut mask = Vec::with_capacity(mask_size)
    for i in 0..mask_size {
        mask.push(0 as u8)
    }
    let mut y: i32 = 0
    while y < height {
        let mut x: i32 = 0
        while x < width {
            let idx = (x + y * width) as usize
            let color_id = mask[idx]
            if color_id == 0 {
                x = x + 1
                continue
            }
            let next_idx = (x + 1 + y * width) as usize
            if mask[next_idx] == color_id {
                x = x + 1
            } else {
                break
            }
            x = x + 1
        }
        y = y + 1
    }
    return 0
}
"#;
    let rs = test_utils::compile_single(src);
    assert!(
        !rs.contains("&mask["),
        "Copy u8 slice must not use &mask[idx] when vec element type unknown; got:\n{}",
        rs
    );
    let (ok, err) = rustc_check(&rs);
    assert!(ok, "rustc failed:\n{}\n\n{}", err, rs);
}

#[test]
fn test_f64_literal_times_f32_cast() {
    let src = r#"
pub fn sphere_phi(ring: i32, rings: i32) -> f32 {
    let phi = 3.14159265359 * ring as f32 / rings as f32
    return phi
}
"#;
    let rs = test_utils::compile_single(src);
    let (ok, err) = rustc_check(&rs);
    assert!(ok, "mixed float E0277:\n{}\n\n{}", err, rs);
}

#[test]
fn test_f32_chain_plus_f64_literal() {
    let src = r#"
pub fn wave(seed: i32) -> f32 {
    let s = (seed as f32 * 0.1).sin() * 0.5 + 0.5
    return s
}
"#;
    let rs = test_utils::compile_single(src);
    let (ok, err) = rustc_check(&rs);
    assert!(ok, "f32 + f64 literal:\n{}\n\n{}", err, rs);
}
