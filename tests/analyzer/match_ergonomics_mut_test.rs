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

fn compile_wj(source: &str, args: &[&str]) -> String {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("test.wj"), source).unwrap();

    let out_dir = dir.path().join("build");
    std::fs::create_dir_all(&out_dir).unwrap();

    let wj = std::env::current_dir().unwrap().join("target/release/wj");

    let mut cmd = Command::new(&wj);
    cmd.arg("build")
        .arg(src_dir.join("test.wj"))
        .arg("--output")
        .arg(&out_dir);
    for a in args {
        cmd.arg(a);
    }
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::read_to_string(out_dir.join("test.rs")).unwrap()
}

fn rustc_check(code: &str) -> Result<(), String> {
    let dir = tempfile::tempdir().unwrap();
    let rs_file = dir.path().join("test.rs");
    std::fs::write(&rs_file, code).unwrap();

    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg(&rs_file)
        .arg("-o")
        .arg(dir.path().join("test"))
        .output()
        .unwrap();

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Bug 2: if let Some(x) = self.opt in &mut self method should not bind x as &mut T for Copy types
#[test]
fn test_if_let_copy_field_in_mut_method() {
    let source = r#"
struct Foo {
    forced: Option<i32>,
    active: i32,
}

impl Foo {
    pub fn update(self) {
        if let Some(level) = self.forced {
            self.active = level
        }
    }
}
"#;

    let generated = compile_wj(source, &[]);

    // The generated code must compile with rustc (no E0308 &mut i32 vs i32)
    let result = rustc_check(&generated);
    assert!(
        result.is_ok(),
        "Generated code failed to compile: {}",
        result.unwrap_err()
    );

    // Verify self.active = level works (level should be i32, not &mut i32)
    assert!(
        !generated.contains("&mut self.forced"),
        "Should not use &mut on Copy Option field. Generated:\n{}",
        generated
    );
}

/// Bug 3: *kf.position.x -- f32 cannot be dereferenced
/// The codegen should NOT add * to Copy field access chains
#[test]
fn test_no_deref_on_copy_field_chain() {
    let source = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

struct Keyframe {
    position: Vec3,
}

pub fn extract_position(kf: Keyframe) -> (f32, f32, f32) {
    (kf.position.x, kf.position.y, kf.position.z)
}
"#;

    let generated = compile_wj(source, &[]);

    // Must NOT contain *kf.position.x -- f32 cannot be dereferenced
    assert!(
        !generated.contains("*kf.position.x"),
        "Should not deref Copy field chain. Generated:\n{}",
        generated
    );

    let result = rustc_check(&generated);
    assert!(
        result.is_ok(),
        "Generated code failed to compile: {}",
        result.unwrap_err()
    );
}

/// Bug 4: &mut u32 == u32 -- comparison through &mut reference
#[test]
fn test_compare_mut_ref_with_value() {
    let source = r#"
struct Editor {
    selected: Vec<u32>,
}

impl Editor {
    pub fn has_selection(self, bone_id: u32) -> bool {
        for sel in self.selected {
            if sel == bone_id {
                return true
            }
        }
        false
    }
}
"#;

    let generated = compile_wj(source, &[]);

    let result = rustc_check(&generated);
    assert!(
        result.is_ok(),
        "Generated code failed to compile: {}",
        result.unwrap_err()
    );
}
