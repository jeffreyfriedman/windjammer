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

use std::path::Path;
use std::process::Command;

fn compile_wj_to_rust(source: &str) -> String {
    let test_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let test_dir = std::env::temp_dir().join(format!("wj_unknown_method_mut_test_{}", test_id));
    let _ = std::fs::remove_dir_all(&test_dir);
    let _ = std::fs::create_dir_all(&test_dir);

    let input_file = test_dir.join("test_input.wj");
    std::fs::write(&input_file, source).unwrap();

    let wj_binary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let _output = Command::new(&wj_binary)
        .arg("build")
        .arg("--no-cargo")
        .arg("test_input.wj")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj compiler");

    // Try multiple output locations
    for candidate in &[
        test_dir.join("build").join("test_input.rs"),
        test_dir.join("test_input.rs"),
    ] {
        if candidate.exists() {
            return std::fs::read_to_string(candidate).unwrap_or_default();
        }
    }
    // Fallback: find any .rs file
    for dir in &[test_dir.join("build"), test_dir.clone()] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map(|x| x == "rs").unwrap_or(false) {
                    return std::fs::read_to_string(entry.path()).unwrap_or_default();
                }
            }
        }
    }
    String::from("NO RS FILE FOUND")
}

/// BUG: When a non-self parameter has an unknown method called on it (method not in
/// SignatureRegistry or method_registry), the compiler should infer &mut because the
/// method might mutate the receiver. Currently it defaults to & (Borrowed), causing
/// E0596 "cannot borrow `*param` as mutable, as it is behind a `&` reference".
///
/// Real example from game dogfooding:
///   fn add_station_mesh_primitives(renderer: VoxelGPURenderer) {
///       renderer.add_primitive(...)  // add_primitive takes &mut self
///   }
/// Generated: pub fn add_station_mesh_primitives(renderer: &VoxelGPURenderer)
/// Should be: pub fn add_station_mesh_primitives(renderer: &mut VoxelGPURenderer)
#[test]
fn test_unknown_method_on_param_infers_mut() {
    let source = r#"
pub struct Renderer {
    pub count: i32,
}

impl Renderer {
    pub fn add_item(self, x: f32, y: f32) {
        self.count = self.count + 1
    }
}

pub fn setup(renderer: Renderer) {
    renderer.add_item(1.0, 2.0)
    renderer.add_item(3.0, 4.0)
}
"#;
    let output = compile_wj_to_rust(source);

    assert!(
        output.contains("renderer: &mut Renderer"),
        "Parameter with mutating method calls should be inferred as &mut. Got:\n{}",
        output
    );
}

/// When a parameter only has known-readonly methods called (like .len(), .is_empty()),
/// it should stay as & (Borrowed).
#[test]
fn test_known_readonly_method_stays_borrowed() {
    let source = r#"
pub struct Data {
    pub items: Vec<i32>,
}

impl Data {
    pub fn len(self) -> i32 {
        self.items.len() as i32
    }
}

pub fn count_items(data: Data) -> i32 {
    data.len()
}
"#;
    let output = compile_wj_to_rust(source);

    assert!(
        output.contains("data: &Data"),
        "Parameter with only readonly method calls should be &. Got:\n{}",
        output
    );
}

/// When a non-self parameter has a method called on it that is NOT in the
/// SignatureRegistry AND the param type is unknown, conservatively infer &mut.
/// When the param type IS known (user type), rely on multi-pass convergence
/// and default to &self (safe for single-pass, correct in multi-pass).
#[test]
fn test_unknown_method_on_typed_param_defaults_borrowed() {
    let source = r#"
pub struct Grid {
    pub name: string,
}

pub fn process(grid: Grid) {
    grid.some_completely_unknown_method(42)
}
"#;
    let output = compile_wj_to_rust(source);

    assert!(
        output.contains("grid: &Grid"),
        "Unknown method on typed user param defaults to borrowed (multi-pass will refine). Got:\n{}",
        output
    );
}

/// When a parameter is only used in field reads (no method calls), it should stay borrowed.
/// Using a non-Copy field (String) ensures the struct is non-Copy and not used in arithmetic.
#[test]
fn test_field_read_only_stays_borrowed() {
    let source = r#"
pub struct Config {
    pub name: string,
    pub enabled: bool,
}

pub fn describe(config: Config) -> string {
    config.name
}
"#;
    let output = compile_wj_to_rust(source);

    // config.name is returned so config needs to be owned (or at least borrowed with clone)
    // This test verifies that field-read-only params aren't forced to &mut
    assert!(
        !output.contains("config: &mut Config"),
        "Parameter with only field reads should NOT be &mut. Got:\n{}",
        output
    );
}
