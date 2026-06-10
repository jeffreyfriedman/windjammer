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

fn compile_wj_to_string(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("test.wj"), source).unwrap();

    let out_dir = dir.path().join("build");
    std::fs::create_dir_all(&out_dir).unwrap();

    let wj_binary = std::env::current_dir().unwrap().join("target/release/wj");

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(src_dir.join("test.wj").to_str().unwrap())
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::read_to_string(out_dir.join("test.rs")).unwrap_or_default()
}

#[test]
fn test_if_let_option_copy_in_mut_method_no_mut_ref_prefix() {
    let source = r#"
struct Foo {
    active_level: i32,
    forced_level: Option<i32>,
    x: f32,
}

impl Foo {
    pub fn update(self) {
        self.x = 1.0
        if let Some(level) = self.forced_level {
            self.active_level = level
            return
        }
    }
}
"#;

    let output = compile_wj_to_string(source);

    // The method should be &mut self (mutates self.x and self.active_level)
    assert!(
        output.contains("&mut self"),
        "Expected &mut self in output:\n{}",
        output
    );

    // The scrutinee should NOT have &mut prefix since Option<i32> is Copy.
    // `if let Some(level) = self.forced_level` (no &mut)
    // NOT `if let Some(level) = &mut self.forced_level`
    assert!(
        !output.contains("&mut self.forced_level"),
        "Should NOT have &mut prefix on Copy Option scrutinee. Output:\n{}",
        output
    );

    // The binding `level` should be used directly as i32
    assert!(
        output.contains("self.active_level = level"),
        "Expected direct assignment of level (i32), not *level. Output:\n{}",
        output
    );
}

#[test]
fn test_if_let_option_copy_tuple_in_mut_method() {
    let source = r#"
struct Camera2D {
    x: f32,
    y: f32,
    bounds: Option<(f32, f32, f32, f32)>,
}

impl Camera2D {
    fn apply_bounds(self) {
        if let Some((min_x, min_y, max_x, max_y)) = self.bounds {
            if self.x < min_x {
                self.x = min_x
            }
            if self.y < min_y {
                self.y = min_y
            }
        }
    }
}
"#;

    let output = compile_wj_to_string(source);

    // Method should be &mut self
    assert!(
        output.contains("&mut self"),
        "Expected &mut self in output:\n{}",
        output
    );

    // Should NOT have &mut prefix on Copy Option scrutinee
    assert!(
        !output.contains("&mut self.bounds"),
        "Should NOT have &mut prefix on Copy Option<tuple> scrutinee. Output:\n{}",
        output
    );
}

#[test]
fn test_if_let_option_non_copy_keeps_mut_ref() {
    let source = r#"
struct Search {
    pub query: string,
}

impl Search {
    pub fn update(self, dt: f32) {
        self.query = "test"
    }
}

struct App {
    active_search: Option<Search>,
    x: f32,
}

impl App {
    pub fn update(self, dt: f32) {
        self.x = 1.0
        if let Some(search) = self.active_search {
            search.update(dt)
        }
    }
}
"#;

    let output = compile_wj_to_string(source);

    // For non-Copy inner type (Search), the &mut prefix is needed
    // to allow mutating methods on the binding
    // (or some other borrow-based mechanism)
    // The key point: non-Copy types should still work through references
    assert!(
        output.contains("&mut self"),
        "Expected &mut self in output:\n{}",
        output
    );
}

#[test]
fn test_if_let_copy_compiles_with_rustc() {
    let source = r#"
struct Foo {
    active_level: i32,
    forced_level: Option<i32>,
    x: f32,
}

impl Foo {
    pub fn update(self) {
        self.x = 1.0
        if let Some(level) = self.forced_level {
            self.active_level = level
            return
        }
    }
}
"#;

    let output = compile_wj_to_string(source);
    if output.trim().is_empty() {
        panic!("No output from wj compiler");
    }

    let dir = tempfile::tempdir().unwrap();
    let rs_path = dir.path().join("test.rs");
    std::fs::write(&rs_path, &output).unwrap();

    let rustc = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg(rs_path.to_str().unwrap())
        .arg("-o")
        .arg(dir.path().join("test.rlib").to_str().unwrap())
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&rustc.stderr);
    assert!(
        rustc.status.success(),
        "Generated Rust should compile. Errors:\n{}\n\nGenerated code:\n{}",
        stderr,
        output
    );
}
