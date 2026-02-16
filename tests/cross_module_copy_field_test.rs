/// TDD Test: No .clone() on Copy fields in MULTI-FILE builds
///
/// Bug: When compiling a multi-file project where struct `ItemStack` (with `quantity: i32`)
/// is defined in `item.wj` and used in `trading.wj` via borrowed iterator:
///   `for stack in &stacks { ... stack.quantity ... }`
/// the compiler adds `.clone()` to `stack.quantity` because `struct_field_types`
/// only contains structs from the current file. Cross-module structs are unknown,
/// so type inference fails and the name-based heuristic doesn't include "quantity".
///
/// Root Cause: CodeGenerator.struct_field_types is populated in generate_struct(),
/// which only runs for structs in the current file's program. Imported struct types
/// are invisible during code generation.
///
/// Expected: Copy-type fields (i32, f32, bool, usize) should NOT have .clone()
/// even in multi-file builds where the struct is defined in another module.

use std::io::Write;
use std::process::Command;

fn compile_wj_project(files: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = dir.path().join("src");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&out_dir).unwrap();

    // Write all source files
    for (name, content) in files {
        let file_path = src_dir.join(name);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    // Compile the project (multi-file)
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            src_dir.to_str().unwrap(),
            "--no-cargo",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("Compilation stderr:\n{}", stderr);
        eprintln!("Compilation stdout:\n{}", stdout);
    }

    // Read all generated .rs files
    let mut results = std::collections::HashMap::new();
    fn collect_rs_files(
        dir: &std::path::Path,
        base: &std::path::Path,
        results: &mut std::collections::HashMap<String, String>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_rs_files(&path, base, results);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let relative = path.strip_prefix(base).unwrap().to_string_lossy().to_string();
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        results.insert(relative, content);
                    }
                }
            }
        }
    }
    collect_rs_files(&out_dir, &out_dir, &mut results);

    results
}

#[test]
fn test_cross_module_no_clone_on_i32_field_via_borrowed_iter() {
    // Multi-file project:
    // - item.wj defines ItemStack { item: Item, quantity: i32 }
    // - trading.wj imports item and iterates: for stack in &stacks { stack.quantity }
    let files = &[
        (
            "mod.wj",
            r#"
pub mod item
pub mod trading
"#,
        ),
        (
            "item.wj",
            r#"
pub struct Item {
    pub id: string,
    pub name: string,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

impl ItemStack {
    pub fn new(item: Item, quantity: i32) -> ItemStack {
        ItemStack { item: item, quantity: quantity }
    }
}
"#,
        ),
        (
            "trading.wj",
            r#"
use item::ItemStack

pub fn total_quantity(stacks: Vec<ItemStack>) -> i32 {
    let mut total = 0
    for stack in &stacks {
        total = total + stack.quantity
    }
    total
}

pub fn has_enough(stacks: Vec<ItemStack>, min_qty: i32) -> bool {
    for stack in &stacks {
        if stack.quantity < min_qty {
            return false
        }
    }
    true
}
"#,
        ),
    ];

    let generated = compile_wj_project(files);

    // Find the trading.rs output
    let trading_rs = generated
        .iter()
        .find(|(k, _)| k.contains("trading"))
        .map(|(_, v)| v.clone())
        .expect("trading.rs not found in output");

    println!("Generated trading.rs:\n{}", trading_rs);

    // stack.quantity is i32 (Copy) — should NOT be cloned
    assert!(
        !trading_rs.contains("stack.quantity.clone()"),
        "Should not clone i32 field 'quantity' accessed through borrowed iterator in multi-file build.\nGenerated:\n{}",
        trading_rs
    );
}

#[test]
fn test_cross_module_no_clone_on_f32_field_via_borrowed_iter() {
    // Multi-file: Vector2 defined in math.wj, used in physics.wj
    let files = &[
        (
            "mod.wj",
            r#"
pub mod math
pub mod physics
"#,
        ),
        (
            "math.wj",
            r#"
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}
"#,
        ),
        (
            "physics.wj",
            r#"
use math::Vector2

pub fn sum_positions(points: Vec<Vector2>) -> f32 {
    let mut total = 0.0
    for p in &points {
        total = total + p.x + p.y
    }
    total
}
"#,
        ),
    ];

    let generated = compile_wj_project(files);

    let physics_rs = generated
        .iter()
        .find(|(k, _)| k.contains("physics"))
        .map(|(_, v)| v.clone())
        .expect("physics.rs not found in output");

    println!("Generated physics.rs:\n{}", physics_rs);

    // p.x and p.y are f32 (Copy) — should NOT be cloned
    assert!(
        !physics_rs.contains("p.x.clone()"),
        "Should not clone f32 field 'x' in multi-file build.\nGenerated:\n{}",
        physics_rs
    );
    assert!(
        !physics_rs.contains("p.y.clone()"),
        "Should not clone f32 field 'y' in multi-file build.\nGenerated:\n{}",
        physics_rs
    );
}
