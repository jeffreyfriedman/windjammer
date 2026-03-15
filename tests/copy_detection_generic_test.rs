//! Generic Copy Detection Tests
//!
//! Verifies that Copy detection works generically from @derive(Copy) in source,
//! NOT from hardcoded type names. The compiler must NEVER know about
//! application-specific types.
//!
//! Architecture: copy_structs_registry (PASS 0) collects types with @derive(Copy)
//! or all-Copy fields. is_known_copy_type is ONLY for external crate types.

use std::process::Command;

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let dir = std::env::temp_dir().join(format!(
        "wj_copy_gen_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    (rs_content, rustc.status.success())
}

/// Custom struct with @derive(Copy) - should NOT generate *(data)
#[test]
fn test_custom_copy_type_from_derive() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct MyData { value: i32 }

pub fn process(data: MyData) -> i32 {
    data.value
}

pub fn main() {
    let d = MyData { value: 42 };
    let _ = process(d);
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(data)"),
        "Should NOT add *(data) for Copy MyData. Generated:\n{}",
        rs
    );
}

/// Struct without @derive(Copy) - should handle correctly
#[test]
fn test_non_copy_type_no_derive() {
    let source = r#"
pub struct MyData { value: String }

pub fn process(data: &MyData) -> usize {
    data.value.len()
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}

/// Struct with all-Copy fields (no explicit derive) - fixed-point should discover it
#[test]
fn test_implicit_copy_all_primitive_fields() {
    let source = r#"
pub struct Point3D { x: f32, y: f32, z: f32 }

pub fn distance(p: Point3D) -> f32 {
    (p.x * p.x + p.y * p.y + p.z * p.z).sqrt()
}

pub fn main() {
    let p = Point3D { x: 1.0, y: 2.0, z: 3.0 };
    let _ = distance(p);
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}

/// Option<CopyType> in if let - should NOT add wrongful *
#[test]
fn test_option_copy_type_if_let() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct State { id: u32 }

pub struct Container { state: Option<State> }

impl Container {
    pub fn new() -> Container {
        Container { state: None }
    }
    fn update(self) {
        if let Some(s) = self.state {
            let _ = s.id;
        }
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(s)"),
        "Should NOT add *(s) for Copy State. Generated:\n{}",
        rs
    );
}

/// Nested Copy struct - both should be in registry
#[test]
fn test_nested_copy_structs() {
    let source = r#"
@derive(Copy, Clone)
pub struct Vec2 { x: f32, y: f32 }

@derive(Copy, Clone)
pub struct Transform { pos: Vec2, scale: f32 }

pub fn get_x(t: Transform) -> f32 {
    t.pos.x
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}

/// Unit enum - always Copy
#[test]
fn test_unit_enum_copy() {
    let source = r#"
pub enum Direction { North, South, East, West }

pub fn is_north(d: Direction) -> bool {
    match d {
        Direction::North => true,
        _ => false,
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}
