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

// TDD: Local variable auto-mutability inference
//
// When a local variable is initialized from a constructor and then used
// with a &mut self method call, the codegen should emit `let mut` even
// if the Windjammer source doesn't explicitly write `let mut`.
//
// Example:
//   let voxelizer = CsgVoxelizer::new(scene)
//   voxelizer.voxelize(...)  // voxelize takes &mut self
//
// Generated Rust should be: let mut voxelizer = CsgVoxelizer::new(scene);

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_local_var_auto_mut_for_mut_method() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("processor.wj"),
        r#"
pub struct Processor {
    pub count: i32
}

impl Processor {
    pub fn new() -> Processor {
        Processor { count: 0 }
    }

    pub fn process(self, value: i32) -> i32 {
        self.count = self.count + value
        self.count
    }
}

pub fn run_processor() -> i32 {
    let proc = Processor::new()
    proc.process(42)
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let generated = std::fs::read_to_string(build.join("processor.rs")).unwrap();
    assert!(
        generated.contains("let mut proc"),
        "Local variable used with &mut self method should get 'let mut'. Generated:\n{}",
        generated
    );
}

#[test]
fn test_local_var_auto_mut_in_assignment_rhs() {
    // Pattern: self.field = local_var.mutating_method(...)
    // The variable appears on the RHS of an assignment, not as
    // a standalone expression statement.
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("voxelizer.wj"),
        r#"
pub struct Grid {
    pub data: i32
}

pub struct Voxelizer {
    pub state: i32
}

impl Voxelizer {
    pub fn new(val: i32) -> Voxelizer {
        Voxelizer { state: val }
    }

    pub fn voxelize(self) -> Grid {
        self.state = self.state + 1
        Grid { data: self.state }
    }
}

pub struct Demo {
    pub grid: Grid
}

impl Demo {
    pub fn init(self) {
        let v = Voxelizer::new(10)
        self.grid = v.voxelize()
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let generated = std::fs::read_to_string(build.join("voxelizer.rs")).unwrap();
    assert!(
        generated.contains("let mut v"),
        "Variable used with &mut self method in assignment RHS should get 'let mut'. Generated:\n{}",
        generated
    );
}

#[test]
fn test_local_var_auto_mut_cross_file() {
    // Cross-file scenario: Voxelizer defined in one file, used in another
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("voxelizer.wj"),
        r#"
pub struct VoxelGrid {
    pub data: i32
}

pub struct CsgVoxelizer {
    pub state: i32
}

impl CsgVoxelizer {
    pub fn new(val: i32) -> CsgVoxelizer {
        CsgVoxelizer { state: val }
    }

    pub fn voxelize(self, size: i32) -> VoxelGrid {
        self.state = self.state + size
        VoxelGrid { data: self.state }
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("demo.wj"),
        r#"
use crate::voxelizer::CsgVoxelizer
use crate::voxelizer::VoxelGrid

pub struct Demo {
    pub grid: VoxelGrid
}

impl Demo {
    pub fn init(self) {
        let v = CsgVoxelizer::new(10)
        self.grid = v.voxelize(64)
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let generated = std::fs::read_to_string(build.join("demo.rs")).unwrap();
    assert!(
        generated.contains("let mut v"),
        "Cross-file: variable used with &mut self method should get 'let mut'. Generated:\n{}",
        generated
    );
}
