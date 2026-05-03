// TDD: Cross-file auto-borrow at call sites
//
// Bug: When file A defines `fn process(data: SomeStruct)` and the analyzer
// infers `data` as Borrowed (generates `data: &SomeStruct`), call sites in
// file B should automatically add `&` to the argument. But the global registry
// built during multipass analysis may have stale ownership (Owned) if the
// final analysis updated it to Borrowed but the global_registry wasn't refreshed.
//
// Root Cause: The final analysis per-file registries are only merged with the
// global registry for the FILE BEING GENERATED, not for OTHER files. So when
// generating file B, the global_registry still has the multipass version of
// file A's signatures.
//
// Fix: After all final analyses complete, rebuild the global registry from
// all per-file final registries, then use that for codegen.

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_cross_file_auto_borrow_at_call_site() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(src.join("rendering")).unwrap();

    // File 1: Defines a non-Copy struct and a method that takes it read-only
    std::fs::write(
        src.join("rendering/renderer.wj"),
        r#"
pub struct Palette {
    pub colors: Vec<f32>
}

pub struct Renderer {
    pub active: bool
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer { active: true }
    }

    // palette is only READ here → analyzer should infer Borrowed
    // Generated Rust: fn upload(palette: &Palette)
    pub fn upload(self, palette: Palette) {
        let count = palette.colors.len()
        self.active = count > 0
    }
}
"#,
    )
    .unwrap();

    // File 2: Calls Renderer::upload, passing self.palette
    // The generated Rust MUST add & to the argument: self.renderer.upload(&self.palette)
    std::fs::write(
        src.join("game.wj"),
        r#"
use crate::rendering::renderer::Palette
use crate::rendering::renderer::Renderer

pub struct Game {
    pub renderer: Renderer,
    pub palette: Palette
}

impl Game {
    pub fn new() -> Game {
        Game {
            renderer: Renderer::new(),
            palette: Palette { colors: Vec::new() }
        }
    }

    pub fn refresh(self) {
        self.renderer.upload(self.palette)
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    // Check that the call site in game.rs adds & to palette
    let game_rs = std::fs::read_to_string(build.join("game.rs")).unwrap();
    assert!(
        game_rs.contains("&self.palette"),
        "Call site should auto-borrow palette with &. Generated:\n{}",
        game_rs
    );

    // Verify that renderer.rs generates &Palette for the parameter
    let renderer_rs = std::fs::read_to_string(build.join("rendering/renderer.rs")).unwrap();
    assert!(
        renderer_rs.contains("&Palette"),
        "Renderer::upload should take &Palette. Generated:\n{}",
        renderer_rs
    );
}
