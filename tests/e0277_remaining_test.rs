//! TDD: Remaining E0277 from dogfooding (trait bound / comparison / mixed float).
//!
//! Covers:
//! 1. Tuple destructuring from `&vec[i]` → `&String == String` (dialogue get_relationship)
//! 2. `Vec::with_capacity` + `u8` indexing → must not emit `&mask[i]` when element is Copy-in-practice
//! 3. Mixed f32/f64: f64 literal × `as f32` and f32 chain + f64 literal

use std::process::Command;
use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);
    assert!(
        float_inference.errors.is_empty(),
        "{:?}",
        float_inference.errors
    );

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).expect("analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

fn rustc_check(rs: &str) -> (bool, String) {
    let mut last_err = String::new();
    for attempt in 0u32..3 {
        let dir = std::env::temp_dir().join(format!(
            "e0277_{}_{}_{}_{:?}",
            std::process::id(),
            attempt,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("crate_root.rs");
        std::fs::write(&p, rs).unwrap();
        let out = Command::new("rustc")
            .args([
                "--crate-type",
                "lib",
                "--edition",
                "2021",
                "--crate-name",
                &format!("e0277_{}", attempt),
                p.to_str().unwrap(),
            ])
            .output()
            .expect("rustc");
        last_err = String::from_utf8_lossy(&out.stderr).into_owned();
        let _ = std::fs::remove_dir_all(&dir);
        if out.status.success() {
            return (true, last_err);
        }
        if !last_err.contains("failed to build archive") && !last_err.contains("failed to open object file")
        {
            return (false, last_err);
        }
    }
    (false, last_err)
}

#[test]
fn test_tuple_destructure_string_compare_owned_param() {
    let src = r#"
pub struct DialogueState {
    pub relationships: Vec<(String, i32)>,
}

impl DialogueState {
    pub fn get_relationship(self, npc: String) -> i32 {
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
    let rs = compile_and_get_rust(src);
    let (ok, err) = rustc_check(&rs);
    assert!(
        ok,
        "E0277 or other rustc error:\n{}\n\n{}",
        err,
        rs
    );
    assert!(
        rs.contains("== &npc") || rs.contains("== & npc"),
        "expected & on owned String param for PartialEq; got:\n{}",
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
    let rs = compile_and_get_rust(src);
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
    let rs = compile_and_get_rust(src);
    let (ok, err) = rustc_check(&rs);
    assert!(
        ok,
        "mixed float E0277:\n{}\n\n{}",
        err,
        rs
    );
}

#[test]
fn test_f32_chain_plus_f64_literal() {
    let src = r#"
pub fn wave(seed: i32) -> f32 {
    let s = (seed as f32 * 0.1).sin() * 0.5 + 0.5
    return s
}
"#;
    let rs = compile_and_get_rust(src);
    let (ok, err) = rustc_check(&rs);
    assert!(ok, "f32 + f64 literal:\n{}\n\n{}", err, rs);
}
