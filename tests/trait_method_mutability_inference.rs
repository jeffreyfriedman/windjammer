/// TDD Test: Trait Method Receiver Mutability Inference
///
/// **Windjammer Philosophy: Compiler Does the Hard Work**
///
/// Problem: Trait methods should NOT require explicit `&mut self` syntax.
/// The compiler should infer mutability based on how `self` is used.
///
/// Example:
/// ```windjammer
/// pub trait Counter {
///     fn increment(self)    // Just `self`, no `&mut`
/// }
///
/// impl Counter for MyCounter {
///     fn increment(self) {
///         self.count = self.count + 1  // Mutates self → compiler infers &mut self
///     }
/// }
/// ```
///
/// The generated Rust should have:
/// - `fn increment(&mut self)` in trait definition
/// - `fn increment(&mut self)` in implementation
///
/// This is **automatic ownership inference** for trait methods.

use std::fs;
use std::process::Command;

#[test]
fn test_trait_method_infers_mut_self_when_mutating() {
    let source = r#"
pub trait Counter {
    fn increment(self)
}

pub struct MyCounter {
    pub count: u32,
}

impl Counter for MyCounter {
    fn increment(self) {
        self.count = self.count + 1
    }
}

fn main() {
    let mut counter = MyCounter { count: 0 }
    counter.increment()
    counter.increment()
    assert_eq(counter.count, 2)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_trait_mut_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    // Compile with windjammer
    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run windjammer compiler");

    assert!(
        output.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Read generated Rust code
    let rs_file = out_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rs_file)
        .expect("Failed to read generated Rust file");

    // Trait definition should have &mut self
    assert!(
        rust_code.contains("fn increment(&mut self)"),
        "Trait method should infer &mut self, got:\n{}",
        rust_code
    );

    // Implementation should also have &mut self
    let has_impl = rust_code.contains("impl Counter for MyCounter");
    let has_mut_method = rust_code[rust_code.find("impl Counter for MyCounter").unwrap_or(0)..]
        .contains("fn increment(&mut self)");
    
    assert!(
        has_impl && has_mut_method,
        "Implementation should also have &mut self, got:\n{}",
        rust_code
    );

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_trait_method_infers_ref_self_when_readonly() {
    let source = r#"
pub trait Reader {
    fn get_value(self) -> u32
}

pub struct MyReader {
    pub value: u32,
}

impl Reader for MyReader {
    fn get_value(self) -> u32 {
        self.value
    }
}

fn main() {
    let reader = MyReader { value: 42 }
    assert_eq(reader.get_value(), 42)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_trait_ref_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("main.wj");
    fs::write(&wj_file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args([
            "build",
            "--target",
            "rust",
            "--no-cargo",
            wj_file.to_str().unwrap(),
        ])
        .current_dir(std::env::current_dir().unwrap())
        .output()
        .expect("Failed to run windjammer compiler");

    assert!(
        output.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_file = test_dir.join("build/main.rs");
    let rust_code = fs::read_to_string(&rs_file)
        .expect("Failed to read generated Rust file");

    // Trait definition should have &self (readonly)
    assert!(
        rust_code.contains("fn get_value(&self) -> u32"),
        "Trait method should infer &self for readonly, got:\n{}",
        rust_code
    );

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_trait_multiple_methods_different_mutability() {
    let source = r#"
pub trait MixedTrait {
    fn read(self) -> u32
    fn write(self, value: u32)
}

pub struct Mixed {
    pub data: u32,
}

impl MixedTrait for Mixed {
    fn read(self) -> u32 {
        self.data
    }
    
    fn write(self, value: u32) {
        self.data = value
    }
}

fn main() {
    let mut obj = Mixed { data: 10 }
    assert_eq(obj.read(), 10)
    obj.write(20)
    assert_eq(obj.read(), 20)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_trait_mixed_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("main.wj");
    fs::write(&wj_file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args([
            "build",
            "--target",
            "rust",
            "--no-cargo",
            wj_file.to_str().unwrap(),
        ])
        .current_dir(std::env::current_dir().unwrap())
        .output()
        .expect("Failed to run windjammer compiler");

    assert!(
        output.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_file = test_dir.join("build/main.rs");
    let rust_code = fs::read_to_string(&rs_file)
        .expect("Failed to read generated Rust file");

    // read() should be &self, write() should be &mut self
    assert!(
        rust_code.contains("fn read(&self) -> u32"),
        "read() should infer &self, got:\n{}",
        rust_code
    );
    
    assert!(
        rust_code.contains("fn write(&mut self, value: u32)"),
        "write() should infer &mut self, got:\n{}",
        rust_code
    );

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_render_port_trait_infers_correctly() {
    // Real-world test case from render_port.wj
    let source = r#"
pub struct CameraData {
    pub view: f32,
}

pub trait RenderPort {
    fn set_camera(self, camera: CameraData)
    fn get_output(self) -> u32
}

pub struct MockRenderer {
    pub camera_set: bool,
    pub output: u32,
}

impl RenderPort for MockRenderer {
    fn set_camera(self, _camera: CameraData) {
        self.camera_set = true
    }
    
    fn get_output(self) -> u32 {
        self.output
    }
}

fn main() {
    let mut renderer = MockRenderer { camera_set: false, output: 42 }
    renderer.set_camera(CameraData { view: 1.0 })
    assert_eq(renderer.camera_set, true)
    assert_eq(renderer.get_output(), 42)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_render_port_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("main.wj");
    fs::write(&wj_file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args([
            "build",
            "--target",
            "rust",
            "--no-cargo",
            wj_file.to_str().unwrap(),
        ])
        .current_dir(std::env::current_dir().unwrap())
        .output()
        .expect("Failed to run windjammer compiler");

    assert!(
        output.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_file = test_dir.join("build/main.rs");
    let rust_code = fs::read_to_string(&rs_file)
        .expect("Failed to read generated Rust file");

    // set_camera mutates, should be &mut self
    assert!(
        rust_code.contains("fn set_camera(&mut self, camera: CameraData)"),
        "set_camera should infer &mut self, got:\n{}",
        rust_code
    );
    
    // get_output is readonly, should be &self
    assert!(
        rust_code.contains("fn get_output(&self) -> u32"),
        "get_output should infer &self, got:\n{}",
        rust_code
    );

    let _ = fs::remove_dir_all(&test_dir);
}
