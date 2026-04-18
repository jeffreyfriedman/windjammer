/// TDD: pub use relative paths should stay relative, absolute paths stay absolute.
///
/// In a module file (mod.wj), `pub use sub_module_a::TypeA` references a child
/// module declared in the same file.  The generated Rust should use `self::` to
/// keep the path relative, not rewrite to `crate::`.

use std::fs;
use std::process::Command;

fn compile_single(source: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let wj = dir.path().join("test.wj");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(&wj, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--no-cargo", "--target", "rust"])
        .arg(&wj)
        .arg("--output")
        .arg(&out)
        .output()
        .expect("Failed to run wj compiler");

    assert!(
        output.status.success(),
        "Compilation failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::read_to_string(out.join("test.rs")).expect("Failed to read generated .rs file")
}

#[test]
fn test_pub_use_relative_paths() {
    let source = r#"
pub mod sub_module_a
pub mod sub_module_b

pub use sub_module_a::TypeA
pub use sub_module_b::TypeB
"#;

    let rust_code = compile_single(source);
    println!("Generated Rust:\n{}", rust_code);

    // Inline mod declarations → self:: prefix is correct Rust 2018+
    assert!(
        rust_code.contains("pub use self::sub_module_a::TypeA")
            || rust_code.contains("pub use sub_module_a::TypeA"),
        "pub use should keep relative (possibly self::) path.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("pub use self::sub_module_b::TypeB")
            || rust_code.contains("pub use sub_module_b::TypeB"),
        "pub use should keep relative (possibly self::) path.\nGenerated:\n{}",
        rust_code
    );

    assert!(
        !rust_code.contains("pub use crate::sub_module_a"),
        "Should NOT rewrite to crate:: for child modules.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("pub use crate::sub_module_b"),
        "Should NOT rewrite to crate:: for child modules.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_pub_use_absolute_paths_unchanged() {
    let source = r#"
pub use crate::some_module::TypeA
pub use crate::another::TypeB
"#;

    let rust_code = compile_single(source);
    println!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("pub use crate::some_module::TypeA"),
        "Absolute paths should remain.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("pub use crate::another::TypeB"),
        "Absolute paths should remain.\nGenerated:\n{}",
        rust_code
    );
}
