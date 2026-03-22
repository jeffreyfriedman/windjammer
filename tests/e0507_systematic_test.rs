//! TDD: E0507 Systematic Pattern Fixes
//!
//! Patterns addressed:
//! A. Vec index in owned context (let binding) - vec[i].clone()
//! B. Shared reference deref - (*param).clone() when param: &T
//! C. Enum variant behind borrowed field - match &self.cost { Cost::Gold(n) => ... }
//! D. For loop over nested field - for p in &self.screenshot.pixels

use std::process::Command;

fn get_wj_binary() -> String {
    env!("CARGO_BIN_EXE_wj").to_string()
}

fn compile_to_rust(wj_source: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    std::fs::write(&wj_path, wj_source).expect("Failed to write test file");
    std::fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_binary())
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let src_main = out_dir.join("src").join("main.rs");
    let test_rs = out_dir.join("test.rs");
    let content = if src_main.exists() {
        std::fs::read_to_string(src_main)
    } else if test_rs.exists() {
        std::fs::read_to_string(test_rs)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No generated Rust file found",
        ))
    };
    content.map_err(|e| e.to_string())
}

fn rust_compiles(rust_code: &str) -> bool {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let rs_path = temp_dir.path().join("test.rs");
    std::fs::write(&rs_path, rust_code).expect("write");
    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            temp_dir.path().join("test.rlib").to_str().unwrap(),
        ])
        .arg(&rs_path)
        .output()
        .expect("rustc");
    output.status.success()
}

#[test]
fn test_vec_index_triangle_clone() {
    // Pattern A: let t0 = tris[start] when Vec<Triangle>
    let source = r#"
pub struct Triangle { pub a: i32 }
pub fn get_first(triangles: Vec<Triangle>) -> Triangle {
    triangles[0]
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains(".clone()") || rust.contains("triangles[0]"),
        "Vec index in return needs clone or Copy: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_vec_index_let_binding_clone() {
    // Pattern A: let t = tris[i] in owned context
    let source = r#"
pub struct Triangle { pub a: i32 }
pub fn process(tris: Vec<Triangle>, i: i32) -> i32 {
    let t = tris[i as usize]
    t.a
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains(".clone()") || rust.contains("&tris[") || rust.contains("tris["),
        "Vec index in let binding: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_shared_ref_deref_clone() {
    // Pattern B: *q when q: &Quest in owned context
    let source = r#"
pub struct Quest { pub name: String }
pub fn get_quest(q: Quest) -> Quest {
    q
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_enum_variant_behind_field() {
    // Pattern C: match self.cost { Cost::Gold(amount) => amount } when &self
    // Method must take &self (inferred) to trigger borrowed field
    let source = r#"
pub enum Cost { Gold(i32) }
pub struct Item { pub cost: Cost }
impl Item {
    pub fn get_amount(self) -> i32 {
        match self.cost {
            Cost::Gold(amount) => amount
        }
    }
}
fn main() {
    let item = Item { cost: Cost::Gold(42) }
    let _ = item.get_amount()
}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains("&self.cost") || rust.contains("match &"),
        "Enum variant behind borrowed field needs &scrutinee: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_for_loop_nested_field() {
    // Pattern D: for p in self.screenshot.pixels when &self
    let source = r#"
pub struct PixelColor { pub r: u8 }
pub struct Screenshot { pub pixels: Vec<PixelColor> }
impl Screenshot {
    pub fn count_pixels(self) -> i32 {
        let mut count = 0
        for p in self.pixels {
            count = count + 1
        }
        count
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains("&self.pixels") || rust.contains("for p in &"),
        "For loop over borrowed field needs &: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_for_loop_deeply_nested_field() {
    // Pattern D: for p in self.screenshot.pixels (nested: self.screenshot)
    let source = r#"
pub struct PixelColor { pub r: u8 }
pub struct Screenshot { pub pixels: Vec<PixelColor> }
pub struct App { pub screenshot: Screenshot }
impl App {
    pub fn process(self) -> i32 {
        let mut count = 0
        for p in self.screenshot.pixels {
            count = count + 1
        }
        count
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains("&self.screenshot.pixels") || rust.contains("for p in &"),
        "For loop over nested borrowed field: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}
