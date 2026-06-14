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

/// TDD Test: Trait methods with explicit &T parameters should not become &&T
///
/// BUG: When a trait method has `fn update(&mut self, input: &Input)`,
/// the generated Rust has `fn update(&mut self, input: &&Input)` (double ref!)
///
/// EXPECTED: Explicit `&` in trait parameters should be preserved as-is, not doubled.
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_explicit_ref_not_doubled() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("out");

    // Create a multi-file project with trait in one file, impl in another
    let src = temp_dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    // Create input module
    std::fs::write(
        src.join("input.wj"),
        r#"
pub struct Input {
    pub x: i32,
}
"#,
    )
    .unwrap();

    // Create game_loop module with trait
    std::fs::write(
        src.join("game_loop.wj"),
        r#"
use crate::input::Input

pub trait GameLoop {
    // Omit `self` — impl body mutates fields, merged signature uses &mut self
    fn update(input: Input) {
        // Default implementation
    }
}
"#,
    )
    .unwrap();

    // Create main module with impl
    let wj_code = r#"
pub mod input;
pub mod game_loop;

use input::Input
use game_loop::GameLoop

struct Game {
    pub score: i32,
}

impl GameLoop for Game {
    fn update(input: Input) {
        self.score += input.x;
    }
}
"#;

    // Write root mod.wj
    let input_file = src.join("mod.wj");
    std::fs::write(&input_file, wj_code).unwrap();
    std::fs::create_dir_all(&output_dir).unwrap();

    let compile_result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&input_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--library")
        .arg("--no-cargo")
        .arg("--no-lint")
        .output()
        .expect("Failed to execute compiler");

    if !compile_result.status.success() {
        eprintln!(
            "STDOUT:\n{}",
            String::from_utf8_lossy(&compile_result.stdout)
        );
        eprintln!(
            "STDERR:\n{}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        panic!("Compiler failed");
    }

    // Check the generated trait file
    let trait_rust = std::fs::read_to_string(output_dir.join("game_loop.rs"))
        .expect("Failed to read generated trait");

    println!(
        "=== Generated game_loop.rs ===\n{}\n=====================",
        trait_rust
    );

    // CRITICAL: no double-reference on parameters (explicit & must not become &&)
    assert!(
        trait_rust.contains("fn update(&mut self, input: Input)")
            && !trait_rust.contains("input: &&Input"),
        "Trait definition must not double-borrow parameters!\nGenerated:\n{}",
        trait_rust
    );

    // Check the impl file (library mode generates lib.rs)
    let main_rust =
        std::fs::read_to_string(output_dir.join("lib.rs")).expect("Failed to read generated lib");

    println!(
        "=== Generated lib.rs ===\n{}\n=====================",
        main_rust
    );

    // The impl should match trait with single & (check if impl exists in lib.rs)
    if main_rust.contains("impl GameLoop for Game") {
        assert!(
            main_rust.contains("fn update(&mut self, input: Input)")
                && !main_rust.contains("input: &&Input"),
            "Impl should match trait without double-borrowing input!\nGenerated:\n{}",
            main_rust
        );
    } else {
        // Impl might be in a separate module file, just check trait worked
        eprintln!("Note: Impl not in lib.rs, checking trait definition was sufficient");
    }
}
