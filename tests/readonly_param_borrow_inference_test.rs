/// TDD Test: Read-only non-Copy parameters should be inferred as borrowed (&T)
///
/// Bug: Functions that only READ their non-Copy parameters (Vec, String, Custom structs)
/// generate owned types in the function signature (e.g., `data: Vec<f32>`), but call
/// sites may pass references (e.g., `&self.frame_times`), causing E0308 mismatched types.
///
/// Root Cause: The analyzer's `infer_parameter_ownership` defaults to `Owned` for all
/// parameters, and `build_signature` forces `Owned` for non-Copy types like Vec, String,
/// Custom. This means read-only parameters are never inferred as `Borrowed`.
///
/// Fix: When a parameter is only read (not mutated, returned, stored, iterated, or used
/// in binary ops), infer `Borrowed`. Update `build_signature` to respect `Borrowed` for
/// non-Copy types instead of forcing `Owned`.
///
/// Discovered via dogfooding: windjammer-game-editor has 6+ E0308 errors from this pattern.
/// Files affected: panels/profiler.rs, panels/hierarchy.rs, panels/inspector.rs
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_wj(source: &str) -> (String, String) {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    let stderr = String::from_utf8_lossy(&wj_output.stderr).to_string();
    (generated, stderr)
}

// ============================================================================
// TEST 1: Method with read-only Vec parameter
//
// Real game code (profiler.wj):
//   fn render_graph(self, data: Vec<f32>, ...) -> string { ... reads data ... }
//   self.render_graph(&self.frame_times, ...)
//
// The function only reads data (len, indexing). The generated Rust should have
// `data: &Vec<f32>`, not `data: Vec<f32>`, so the call site `&self.frame_times`
// matches.
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_readonly_vec_param_inferred_as_borrowed() {
    let source = r#"
pub struct Profiler {
    pub frame_times: Vec<f32>,
    pub fps_history: Vec<f32>,
}

impl Profiler {
    pub fn render_graph(self, data: Vec<f32>, label: string) -> string {
        if data.len() < 2 {
            return "No data".to_string()
        }
        let first = data[0]
        format!("{}: {} points, first={}", label, data.len(), first)
    }

    pub fn render(self) -> string {
        let graph1 = self.render_graph(&self.frame_times, "Frame Times".to_string())
        let graph2 = self.render_graph(&self.fps_history, "FPS".to_string())
        format!("{}\n{}", graph1, graph2)
    }
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // render_graph should take data as &Vec<f32> (borrowed) because it only reads
    assert!(
        generated.contains("data: &Vec<f32>"),
        "COMPILER BUG: render_graph only reads 'data' (len, indexing). \
         Should be 'data: &Vec<f32>', not 'data: Vec<f32>'.\n\
         This causes E0308 when call sites pass &self.frame_times.\n\
         Generated:\n{}",
        generated
    );

    // String params stay Owned to avoid &String vs &str mismatches at call sites
    let render_graph_line = generated.lines().find(|l| l.contains("fn render_graph"));
    if let Some(line) = render_graph_line {
        assert!(
            line.contains("label: String"),
            "String params should stay Owned (avoiding &String vs &str mismatch).\n\
             Line: {}",
            line
        );
    }
}

// ============================================================================
// TEST 2: Method with read-only Custom struct parameter
//
// Real game code (hierarchy.wj):
//   fn render_node(self, object: SceneObject, depth: i32) -> string { ... }
//   self.render_node(obj, 0)  // where obj is &SceneObject from map.get()
//
// The function only reads object fields. Should be `object: &SceneObject`.
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_readonly_custom_struct_param_inferred_as_borrowed() {
    let source = r#"
pub struct SceneObject {
    pub id: string,
    pub name: string,
    pub visible: bool,
}

pub struct HierarchyPanel {
    pub filter_text: string,
}

impl HierarchyPanel {
    pub fn render_node(self, object: SceneObject, depth: i32) -> string {
        let indent = depth * 20
        format!("<div style='margin-left:{}px'>{} ({})</div>", indent, object.name, object.id)
    }

    pub fn render(self, objects: Vec<SceneObject>) -> string {
        let mut html = "".to_string()
        for obj in &objects {
            html = html + self.render_node(obj, 0).as_str()
        }
        html
    }
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // render_node should take object as &SceneObject (borrowed) because it only reads fields
    assert!(
        generated.contains("object: &SceneObject"),
        "COMPILER BUG: render_node only reads 'object' fields (name, id). \
         Should be 'object: &SceneObject', not 'object: SceneObject'.\n\
         This causes E0308 when call sites pass &SceneObject from iterators.\n\
         Generated:\n{}",
        generated
    );

    // depth is i32 (Copy type) - should remain owned (pass by value)
    let render_node_line = generated.lines().find(|l| l.contains("fn render_node"));
    if let Some(line) = render_node_line {
        assert!(
            line.contains("depth: i32"),
            "Copy type 'depth: i32' should remain owned (pass by value).\n\
             Line: {}",
            line
        );
    }
}

// ============================================================================
// TEST 3: Parameters that ARE consumed should stay owned
//
// Ensure the fix doesn't break parameters that need to be owned:
// - Parameters stored in struct fields
// - Parameters returned from the function
// - Parameters passed to functions that consume them
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_consumed_params_stay_owned() {
    let source = r#"
pub struct Config {
    pub name: string,
    pub items: Vec<i32>,
}

impl Config {
    pub fn new(name: string, items: Vec<i32>) -> Config {
        Config { name: name, items: items }
    }
}

pub fn get_name(config: Config) -> string {
    config.name
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // Config::new stores both params - they MUST stay owned
    assert!(
        generated.contains("name: String") && !generated.contains("name: &String"),
        "Config::new stores 'name' in struct - must be owned String, not &String.\n\
         Generated:\n{}",
        generated
    );

    // get_name returns config.name (moves it out) - config must be owned or the
    // field access on a reference would need clone. The analyzer should detect
    // that the return value comes from the parameter.
    // NOTE: This could also work as &Config with config.name.clone(), but
    // is_returned should catch this case.
}

// ============================================================================
// TEST 4: String parameters stay owned (avoiding &String vs &str mismatches)
//
// String parameters are a special case. In Rust, &String doesn't accept &str
// literals ("hello"), so borrowing String params would cause type errors at
// call sites passing string literals. We keep String params Owned.
// Future: could generate &str for borrowed String params (more idiomatic Rust).
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_param_stays_owned() {
    let source = r#"
pub struct Logger {
    pub prefix: string,
}

impl Logger {
    pub fn log_message(self, message: string) {
        let output = format!("[{}] {}", self.prefix, message)
    }
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // String params should stay Owned to avoid &String vs &str issues at call sites
    let log_line = generated.lines().find(|l| l.contains("fn log_message"));
    if let Some(line) = log_line {
        assert!(
            line.contains("message: String"),
            "String params should stay Owned (avoiding &String vs &str mismatch).\n\
             Line: {}\n\
             Generated:\n{}",
            line,
            generated
        );
    }
}
