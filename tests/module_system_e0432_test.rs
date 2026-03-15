//! TDD: E0432 Unresolved Import Fix
//!
//! Tests for module path resolution in pub use statements.
//! Root cause: `pub use crate::animation::Animation` in animation/mod.rs fails
//! because crate::animation refers to the current module (circular).
//!
//! Fix: Rewrite crate::<current_module>::X to self::X for nested mod.rs.

use std::fs;
use std::process::Command;

fn compile_project(files: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src_wj");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();

    for (path, content) in files {
        let full_path = src_dir.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, content).unwrap();
    }

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--features",
            "cli",
            "--bin",
            "wj",
            "--",
            "build",
            src_dir.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Compiler failed. stderr:\n{}\nstdout:\n{}",
            stderr, stdout
        );
    }

    let mut result = std::collections::HashMap::new();
    fn read_dir_recursive(
        dir: &std::path::Path,
        base: &std::path::Path,
        result: &mut std::collections::HashMap<String, String>,
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    read_dir_recursive(&path, base, result);
                } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    let rel = path
                        .strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    let content = fs::read_to_string(&path).unwrap();
                    result.insert(rel, content);
                }
            }
        }
    }
    read_dir_recursive(&out_dir, &out_dir, &mut result);
    result
}

/// E0432: pub use with sibling modules must use self:: prefix
/// animation/mod.wj: pub use animation::Animation
/// Generated mod.rs must have: pub use self::animation::Animation
#[test]
fn test_pub_use_sibling_uses_self_prefix() {
    let files = &[
        ("mod.wj", "pub mod animation\n"),
        (
            "animation/mod.wj",
            "pub mod animation\npub mod state\npub use animation::Animation\npub use state::AnimationState\n",
        ),
        ("animation/animation.wj", "pub struct Animation { pub frames: i32 }"),
        ("animation/state.wj", "pub struct AnimationState { pub current: i32 }"),
    ];

    let output = compile_project(files);

    let mod_rs = output
        .get("animation/mod.rs")
        .expect("Should generate animation/mod.rs");

    assert!(
        mod_rs.contains("pub use self::animation::Animation"),
        "Sibling re-export must use self:: prefix. Got:\n{}",
        mod_rs
    );
    assert!(
        mod_rs.contains("pub use self::state::AnimationState"),
        "Sibling re-export must use self:: prefix. Got:\n{}",
        mod_rs
    );
    assert!(
        !mod_rs.contains("pub use crate::animation::Animation"),
        "Must NOT use crate:: for self-reference (E0432). Got:\n{}",
        mod_rs
    );
}

/// E0432: pub use crate::<current_module>::X in nested mod.rs is invalid
/// When mod.wj has pub use crate::animation::Animation, we're IN animation/mod.rs
/// crate::animation = current module. Rewrite to self::animation::Animation
#[test]
fn test_pub_use_crate_current_module_rewritten_to_self() {
    let files = &[
        ("mod.wj", "pub mod animation\n"),
        (
            "animation/mod.wj",
            "pub mod animation\npub use crate::animation::Animation\n",
        ),
        ("animation/animation.wj", "pub struct Animation { pub frames: i32 }"),
    ];

    let output = compile_project(files);

    let mod_rs = output
        .get("animation/mod.rs")
        .expect("Should generate animation/mod.rs");

    assert!(
        mod_rs.contains("pub use self::animation::Animation"),
        "crate::<current> must be rewritten to self::. Got:\n{}",
        mod_rs
    );
    assert!(
        !mod_rs.contains("pub use crate::animation::Animation"),
        "Must NOT output crate::animation in animation/mod.rs (E0432). Got:\n{}",
        mod_rs
    );
}

/// Nested module paths: pub use from sibling
/// math/mod.wj: pub use vec2::Vec2 - vec2 is sibling, use self::vec2::Vec2
#[test]
fn test_nested_module_sibling_re_exports() {
    let files = &[
        ("mod.wj", "pub mod math\n"),
        (
            "math/mod.wj",
            "pub mod vec2\npub mod vec3\npub use vec2::Vec2\npub use vec3::Vec3\n",
        ),
        ("math/vec2.wj", "pub struct Vec2 { pub x: f64, pub y: f64 }"),
        ("math/vec3.wj", "pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }"),
    ];

    let output = compile_project(files);

    let mod_rs = output
        .get("math/mod.rs")
        .expect("Should generate math/mod.rs");

    assert!(
        mod_rs.contains("pub use self::vec2::Vec2"),
        "math/mod.rs must use self::vec2. Got:\n{}",
        mod_rs
    );
    assert!(
        mod_rs.contains("pub use self::vec3::Vec3"),
        "math/mod.rs must use self::vec3. Got:\n{}",
        mod_rs
    );
}
