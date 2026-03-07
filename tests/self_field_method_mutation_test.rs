/// TDD Test: Self ownership inference when calling methods on self.field
///
/// Bug: When render() calls self.renderer.update_camera(camera) and
/// self.renderer.render_frame(), these methods mutate self.renderer
/// (they need &mut self). The analyzer should infer &mut self for render()
/// because calling any mutating method on self.field requires mutable access.
///
/// Root Cause: expression_is_self_field_mutating_method_call only checks
/// is_mutating_method() which is a hardcoded stdlib list. User-defined
/// methods like update_camera(), render_frame(), initialize(), shutdown()
/// are not in that list.
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile_windjammer_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    std::fs::write(&input_file, code).expect("Failed to write source file");

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output = Command::new(&wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.join("build").to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(format!(
            "Windjammer compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated_file = test_dir.join("build/test.rs");
    let generated =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated file");
    Ok(generated)
}

#[test]
fn test_self_field_method_call_propagates_mutability() {
    let code = r#"
pub struct Inner {
    count: u32,
}

impl Inner {
    pub fn new() -> Inner {
        Inner { count: 0 }
    }

    pub fn update_state(self) {
        self.count = self.count + 1
    }
}

pub struct Outer {
    inner: Inner,
}

impl Outer {
    pub fn new() -> Outer {
        Outer { inner: Inner::new() }
    }

    pub fn do_work(self) {
        self.inner.update_state()
    }
}

fn main() {
    let outer = Outer::new()
    outer.do_work()
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains("fn do_work(&mut self)"),
        "do_work() should infer &mut self because it calls self.inner.update_state() which mutates inner.\nGenerated code:\n{}",
        generated
    );
}

#[test]
fn test_self_field_render_frame_propagates_mutability() {
    let code = r#"
pub struct Renderer {
    frame_count: u32,
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer { frame_count: 0 }
    }

    pub fn render_frame(self) {
        self.frame_count = self.frame_count + 1
    }
}

pub struct Demo {
    renderer: Renderer,
    initialized: bool,
}

impl Demo {
    pub fn new() -> Demo {
        Demo { renderer: Renderer::new(), initialized: false }
    }

    pub fn render(self) {
        if !self.initialized { return }
        self.renderer.render_frame()
    }
}

fn main() {
    let demo = Demo::new()
    demo.render()
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains("fn render(&mut self)"),
        "render() should infer &mut self because it calls self.renderer.render_frame() which mutates renderer.\nGenerated code:\n{}",
        generated
    );
}
