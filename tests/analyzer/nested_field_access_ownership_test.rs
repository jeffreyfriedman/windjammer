/// TDD: Nested field access (self.compositor.width) should NOT cause the method
/// to be inferred as taking owned `self`. It should be `&self` or `&mut self`.
///
/// Bug: self.compositor.mesh_render_width where compositor is a non-Copy struct
/// was classified as "moves non-Copy self field", forcing the method to be owned.
/// But accessing a sub-field is a borrow, not a move.
///
/// Fix: Nested field chains (self.a.b) are borrows, only direct `self.field`
/// used standalone is a potential move.
use std::process::Command;

fn compile_wj_to_rs(source: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let src_path = dir.path().join("test.wj");
    std::fs::write(&src_path, source).unwrap();

    let wj = std::env::var("WJ_BINARY").unwrap_or_else(|_| {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest
            .join("target/release/wj")
            .to_string_lossy()
            .to_string()
    });

    let out_dir = dir.path().join("out");
    let output = Command::new(&wj)
        .arg("build")
        .arg(src_path.to_str().unwrap())
        .arg("--no-cargo")
        .arg("-o")
        .arg(out_dir.to_str().unwrap())
        .output()
        .expect("Failed to execute wj");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let rs_path = out_dir.join("test.rs");
    let generated = std::fs::read_to_string(&rs_path).unwrap_or_default();

    (
        output.status.success(),
        generated,
        format!("{}\n{}", stdout, stderr),
    )
}

#[test]
fn test_nested_field_access_infers_borrow_not_owned() {
    let source = r#"
pub struct Inner {
    pub width: i32,
    pub height: i32,
}

pub struct Outer {
    pub inner: Inner,
    pub count: i32,
}

impl Outer {
    pub fn get_area(self) -> i32 {
        self.inner.width * self.inner.height
    }
}
"#;

    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "Compilation should succeed: {}", output);

    // The method should take &self (read-only), not owned self
    assert!(
        generated.contains("fn get_area(&self)"),
        "Method should be inferred as &self, not owned.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_nested_field_access_with_mutation_infers_mut_borrow() {
    let source = r#"
pub struct Config {
    pub value: i32,
}

pub struct System {
    pub config: Config,
    pub active: bool,
}

impl System {
    pub fn update(self) {
        self.active = true
        let v = self.config.value
    }
}
"#;

    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "Compilation should succeed: {}", output);

    // The method mutates self.active, so should be &mut self
    assert!(
        generated.contains("fn update(&mut self)"),
        "Method should be inferred as &mut self.\nGenerated:\n{}",
        generated
    );
}
