// TDD: Test float literal type inference in multi-file builds
//
// Bug: Float inference works for single files but fails in multi-file library builds
// 
// Error: "Type conflict at seq_id=35, 56:33: must be f32 (identifier dt has type f32) 
//         but was inferred as f64"
//
// Root Cause: Constraint solver state management across files OR
//             Literal locations not being tracked correctly across files

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_float_inference_single_file_with_f32_param() {
    // This test PASSES - single file inference works
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    std::fs::write(
        src.join("mira.wj"),
        r#"
pub struct Companion {
    pub health: f32
}

impl Companion {
    pub fn update(self, dt: f32) {
        // dt is f32, so 1000.0 should be inferred as f32
        let frame_time_ms = dt * 1000.0
        self.health = self.health + dt
    }
}
"#,
    )
    .unwrap();
    
    build_project_ext(
        &src.join("mira.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        false, // NOT library mode
        &[],
    )
    .expect("Single file build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("mira.rs")).unwrap();
    
    // ASSERT: 1000.0 should be f32, not f64
    assert!(
        !rust_code.contains("1000.0_f64"),
        "1000.0 should be inferred as f32 (used with f32 parameter dt)"
    );
}

#[test]
fn test_float_inference_multi_file_with_f32_param() {
    // This test FAILS - multi-file inference has state management bugs
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(src.join("companions")).unwrap();
    
    // File 1: companion.wj (defines Companion struct)
    std::fs::write(
        src.join("companions/companion.wj"),
        r#"
pub struct Companion {
    pub health: f32,
    pub name: String
}

impl Companion {
    pub fn new(name: String) -> Companion {
        Companion {
            health: 100.0,
            name: name
        }
    }
}
"#,
    )
    .unwrap();
    
    // File 2: mira.wj (uses Companion, has f32 param with literal)
    std::fs::write(
        src.join("companions/mira.wj"),
        r#"
use crate::companions::companion::Companion

pub struct Mira {
    pub companion: Companion
}

impl Mira {
    pub fn new() -> Mira {
        let companion = Companion::new("Mira")
        Mira { companion: companion }
    }
    
    pub fn update(self, dt: f32) {
        // dt is f32, so 1000.0 should be inferred as f32
        // BUG: In multi-file builds, this literal is incorrectly inferred as f64
        let frame_time_ms = dt * 1000.0
        self.companion.health = self.companion.health + dt
    }
}
"#,
    )
    .unwrap();
    
    // File 3: mod.wj (module root)
    std::fs::write(
        src.join("companions/mod.wj"),
        r#"
pub mod companion
pub mod mira
"#,
    )
    .unwrap();
    
    // Root mod.wj
    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod companions
"#,
    )
    .unwrap();
    
    // Build as library (multi-file)
    build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true, // LIBRARY mode (multi-file)
        &[],
    )
    .expect("Multi-file library build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("companions/mira.rs")).unwrap();
    
    // ASSERT: 1000.0 should be f32, not f64
    assert!(
        !rust_code.contains("1000.0_f64"),
        "1000.0 should be inferred as f32 (used with f32 parameter dt). Found:\n{}",
        rust_code.lines()
            .find(|l| l.contains("1000.0"))
            .unwrap_or("NOT FOUND")
    );
}

#[test]
fn test_float_inference_respects_parameter_types_across_files() {
    // Minimal reproduction: just the failing pattern
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    // File 1
    std::fs::write(
        src.join("a.wj"),
        r#"
pub fn helper(x: f32) -> f32 {
    x * 2.0
}
"#,
    )
    .unwrap();
    
    // File 2 - uses f32 param with literal
    std::fs::write(
        src.join("b.wj"),
        r#"
pub fn main_func(dt: f32) -> f32 {
    dt * 1000.0
}
"#,
    )
    .unwrap();
    
    // Root
    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod a
pub mod b
"#,
    )
    .unwrap();
    
    build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("Build should succeed");
    
    let b_code = std::fs::read_to_string(build.join("b.rs")).unwrap();
    
    // Both literals should be f32
    assert!(
        !b_code.contains("1000.0_f64"),
        "1000.0 in b.wj should be f32 (dt parameter is f32)"
    );
}
