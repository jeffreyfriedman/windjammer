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

// A local declared `let mut` (ADR-0002: local mutability is explicit) and then used with a method whose receiver is
// `&mut self` keeps its `mut` in the generated Rust, in one file and across files. Without the `mut`, `wj check`
// reports WJ0051 (each source is checked both ways below).

fn reports_wj0051(files: &[(&str, &str)], name: &str) -> bool {
    let sources: Vec<(String, String)> = files.iter().map(|(f, t)| (f.to_string(), t.replace(&format!("let mut {name}"), &format!("let {name}")))).collect();
    windjammer::check::local_mutability_sources(&sources).iter().any(|d| d.message.starts_with(&format!("`{name}`")))
}

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
    let mut proc = Processor::new()
    proc.process(42)
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let text = std::fs::read_to_string(src.join("processor.wj")).unwrap();
    assert!(reports_wj0051(&[("processor.wj", &text)], "proc"), "without `mut`, WJ0051");
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
        let mut v = Voxelizer::new(10)
        self.grid = v.voxelize()
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let text = std::fs::read_to_string(src.join("voxelizer.wj")).unwrap();
    assert!(reports_wj0051(&[("voxelizer.wj", &text)], "v"), "without `mut`, WJ0051");
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
        let mut v = CsgVoxelizer::new(10)
        self.grid = v.voxelize(64)
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let lib = std::fs::read_to_string(src.join("voxelizer.wj")).unwrap();
    let demo = std::fs::read_to_string(src.join("demo.wj")).unwrap();
    assert!(reports_wj0051(&[("voxelizer.wj", &lib), ("demo.wj", &demo)], "v"), "without `mut`, WJ0051 across files");
    let generated = std::fs::read_to_string(build.join("demo.rs")).unwrap();
    assert!(
        generated.contains("let mut v"),
        "Cross-file: variable used with &mut self method should get 'let mut'. Generated:\n{}",
        generated
    );
}
