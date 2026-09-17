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
))]

//! P3.345: Void `impl` methods (`debug_renderer.draw_circle_xz`) with `let seg = if segments < 4 …`
//! and `while i < seg { (i + 1) as f32 * step }` must not emit `1_i64` on the counter.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const FIXTURE: &str = r#"
pub struct DebugRenderer {}

impl DebugRenderer {
    pub fn draw_circle_xz(self, cx: f32, cy: f32, cz: f32, radius: f32, segments: i32, r: f32, g: f32, b: f32) {
        let seg = if segments < 4 { 4 } else { segments }
        let step = 6.28318 / (seg as f32)
        let mut i = 0
        while i < seg {
            let a0 = (i as f32) * step
            let a1 = ((i + 1) as f32) * step
            let _x0 = cx + radius * a0.cos()
            let _x1 = cx + radius * a1.cos()
            i = i + 1
        }
    }
}
"#;

#[test]
fn module_file_void_while_i32_seg_counter_must_not_emit_i64() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), FIXTURE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "library build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    assert!(
        !generated.contains("+ 1_i64"),
        "P3.345: void impl while i < seg must not emit i64 literal peers:\n{generated}"
    );
}
