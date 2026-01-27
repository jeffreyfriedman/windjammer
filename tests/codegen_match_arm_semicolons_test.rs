// TDD TEST: Match arm statements must preserve semicolons
//
// BUG: Match arms with function calls that return values are generating
//      code without semicolons, causing type mismatch errors.
//
// ROOT CAUSE: Code generator not preserving statement semicolons in match arms
//
// FIX: Ensure match arm statement generation includes semicolons

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_match_arm_statement_semicolons() {
    let code = r#"
pub fn apply_damage(damage_type: DamageType, amount: i32) {
    match damage_type {
        DamageType::Fire => {
            take_damage(amount * 2);
        },
        DamageType::Ice => {
            take_damage(amount);
        },
        _ => {},
    }
}

pub fn take_damage(amount: i32) -> i32 {
    amount
}

pub enum DamageType {
    Fire,
    Ice,
    Physical,
}

fn main() {
    let dt = DamageType::Fire;
    apply_damage(dt, 10);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");
    fs::write(&wj_file, code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .output()
        .expect("Failed to run wj");

    assert!(
        result.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Read generated Rust
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(rust_file).unwrap();

    // Should have semicolons after take_damage calls
    assert!(
        rust_code.contains("take_damage(amount * 2);"),
        "Missing semicolon after take_damage in Fire arm"
    );
    assert!(
        rust_code.contains("take_damage(amount);"),
        "Missing semicolon after take_damage in Ice arm"
    );

    println!("✅ Generated Rust has correct semicolons in match arms");
}
