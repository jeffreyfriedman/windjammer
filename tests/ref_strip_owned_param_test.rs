use anyhow::Result;
/// TDD Test: Strip explicit & when method parameter expects owned value
///
/// PROBLEM: When WJ source passes `&object.field` to a method that expects an owned parameter,
/// the compiler generates `&object.field` in Rust. But if the method signature says the
/// parameter is owned (e.g., `fn render_transform(self, transform: Transform)`), Rust
/// expects `Transform`, not `&Transform`, causing E0308: "expected Transform, found &Transform".
///
/// WINDJAMMER PHILOSOPHY: The developer shouldn't need to think about &.
/// The compiler should strip `&` when the target parameter is owned.
///
/// Example WJ source:
/// ```
/// fn render_transform(self, transform: Transform) -> string { ... }
/// let result = self.render_transform(&object.transform)
/// ```
///
/// Should generate:
/// ```rust
/// let result = self.render_transform(object.transform);  // & stripped
/// ```
///
/// Should NOT generate:
/// ```rust
/// let result = self.render_transform(&object.transform);  // ❌ E0308!
/// ```
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_strip_ref_when_method_param_is_owned() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_ref_strip_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write a WJ file where &expr is passed to a method expecting owned
    fs::write(
        src_dir.join("main.wj"),
        r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Transform {
    pub position: Vec3,
}

pub struct Inspector {
    pub name: string,
}

impl Inspector {
    pub fn render_transform(self, transform: Transform) -> string {
        format!("pos: {}", transform.position.x)
    }

    pub fn inspect(self, object: Transform) -> string {
        // Explicitly passing &object.position — compiler should strip & since
        // render_transform takes Transform (owned, Copy type)
        self.render_transform(&object)
    }
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "ref-strip-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Compile
    let output_dir = temp_dir.join("src");
    let output = Command::new(get_wj_compiler())
        .args(["build", "--no-cargo"])
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Read the generated Rust file
    let generated = fs::read_to_string(output_dir.join("main.rs")).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated main.rs\nstdout: {}\nstderr: {}",
            stdout, stderr
        )
    });

    // ASSERTION: The & should be stripped in the render_transform call
    // The generated code should have `self.render_transform(object)` NOT `self.render_transform(&object)`
    assert!(
        !generated.contains("self.render_transform(&object)"),
        "Explicit & should be STRIPPED when method param is owned.\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("self.render_transform(object"),
        "Method call should pass owned value (no &) when param is owned.\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ref_strip_compiles_with_rustc() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_ref_strip_compile_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write a WJ file that passes &expr to an owned param — must compile cleanly
    fs::write(
        src_dir.join("main.wj"),
        r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Transform {
    pub position: Vec3,
}

pub struct Inspector {
    pub name: string,
}

impl Inspector {
    pub fn render_transform(self, transform: Transform) -> string {
        format!("pos: {}", transform.position.x)
    }

    pub fn inspect(self, object: Transform) -> string {
        self.render_transform(&object)
    }
}

fn main() {
    let inspector = Inspector { name: "test".to_string() }
    let transform = Transform { position: Vec3 { x: 1.0, y: 2.0, z: 3.0 } }
    let result = inspector.inspect(transform)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "ref-strip-compile-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Compile WITH cargo to verify the generated Rust actually compiles
    let output_dir = temp_dir.join("src");
    let output = Command::new(get_wj_compiler())
        .args(["build"])
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // ASSERTION: The build should succeed (exit code 0)
    assert!(
        output.status.success(),
        "Build should succeed when & is stripped for owned param.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}
