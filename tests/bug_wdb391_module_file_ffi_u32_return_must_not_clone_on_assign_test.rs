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

//! WDB-391: Copy u32 from a helper/FFI return, assigned to a field and reused
//! in `w * h`, must not emit `w.clone()` / `h.clone()`.
//!
//! Product gen/rendering/voxel_gpu_buffers.rs:
//!   let w = gpu::get_screen_width();
//!   self.screen_width = w.clone();
//! WJ source is `self.screen_width = w` then `let pixel_count = w * h`.
//! Distinct from WDB-343 (typed i32 formals) and WDB-344 (f32 locals).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn get_screen_width() -> u32 {
    1280
}

pub fn get_screen_height() -> u32 {
    720
}

pub struct Renderer {
    pub screen_width: u32,
    pub screen_height: u32,
}

impl Renderer {
    pub fn init_gpu(self) {
        let w = get_screen_width()
        let h = get_screen_height()
        self.screen_width = w
        self.screen_height = h
        let pixel_count = w * h
        let _ = pixel_count
    }
}
"#;

#[test]
fn wdb391_module_file_ffi_u32_return_must_not_clone_on_assign() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-391 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-391 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("w.clone()") || rs.contains("h.clone()");
    assert!(
        !bad,
        "WDB-391 RED: Copy u32 helper return cloned on assign/reuse:\n{rs}"
    );
    test.cargo_check().expect("WDB-391 cargo-check");
}

#[test]
fn wdb391_tip_out_game_core_gpu_buffers_must_not_clone_screen_u32() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("rendering/voxel_gpu_buffers.rs"),
        tip.join("voxel_gpu_buffers.rs"),
        game.join("gen/rendering/voxel_gpu_buffers.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//")
                && (t.contains("screen_width = w.clone()")
                    || t.contains("screen_height = h.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-391: voxel_gpu_buffers product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-391 RED: tip/product Copy u32 w.clone()/h.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
