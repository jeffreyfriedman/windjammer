/// TDD Test: No .clone() on Copy fields accessed through BORROWED iterator vars
///
/// Bug: When iterating `for stack in &stacks` where `stacks: Vec<ItemStack>`,
/// and accessing `stack.quantity` (i32, Copy type), the compiler incorrectly
/// adds `.clone()`: `stack.quantity.clone()`.
///
/// Root Cause: The borrowed_iterator_vars logic knows `stack` is borrowed,
/// but the type inference can't resolve the element type of the parameter
/// `stacks: Vec<ItemStack>` → `stack: ItemStack` → `stack.quantity: i32`.
/// Since the type is unknown, the name-based heuristic check at line ~7333
/// doesn't include "quantity", so .clone() is incorrectly added.
///
/// Expected: Copy-type fields (i32, f32, bool, usize) should NOT have .clone()
/// even when accessed through borrowed iterator variables.

use std::io::Write;
use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = dir.path().join("test.wj");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut file = std::fs::File::create(&wj_path).unwrap();
    file.write_all(source.as_bytes()).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "--no-cargo",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let rs_path = out_dir.join("test.rs");
    if rs_path.exists() {
        std::fs::read_to_string(&rs_path).unwrap()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "No output file generated.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
}

#[test]
fn test_no_clone_on_i32_field_via_borrowed_iter() {
    // Exact pattern from windjammer-game rpg/trading.wj:
    // for stack in &stacks { ... stack.quantity ... }
    // stack.quantity is i32 (Copy) — should NOT get .clone()
    let source = r#"
pub struct Item {
    pub id: string,
    pub name: string,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub fn total_quantity(stacks: Vec<ItemStack>) -> i32 {
    let mut total = 0
    for stack in &stacks {
        total = total + stack.quantity
    }
    total
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // stack.quantity is i32 (Copy) — should NOT be cloned
    assert!(
        !generated.contains("stack.quantity.clone()"),
        "Should not clone i32 field 'quantity' accessed through borrowed iterator.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_f32_field_via_borrowed_iter() {
    // Access f32 field through borrowed iterator
    let source = r#"
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub mass: f32,
}

pub fn total_mass(particles: Vec<Particle>) -> f32 {
    let mut total = 0.0
    for p in &particles {
        total = total + p.mass
    }
    total
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // p.mass is f32 (Copy) — should NOT be cloned
    assert!(
        !generated.contains("p.mass.clone()"),
        "Should not clone f32 field 'mass' accessed through borrowed iterator.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_bool_field_via_borrowed_iter() {
    // Access bool field through borrowed iterator
    let source = r#"
pub struct Task {
    pub name: string,
    pub completed: bool,
    pub priority: i32,
}

pub fn count_completed(tasks: Vec<Task>) -> i32 {
    let mut count = 0
    for task in &tasks {
        if task.completed {
            count = count + 1
        }
    }
    count
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // task.completed is bool (Copy) — should NOT be cloned
    assert!(
        !generated.contains("task.completed.clone()"),
        "Should not clone bool field 'completed' accessed through borrowed iterator.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_clone_on_string_field_via_borrowed_iter_is_ok() {
    // String field through borrowed iterator SHOULD still get .clone()
    // because String is NOT Copy
    let source = r#"
pub struct Item {
    pub id: string,
    pub name: string,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub fn collect_names(stacks: Vec<ItemStack>) -> Vec<string> {
    let mut names: Vec<string> = Vec::new()
    for stack in &stacks {
        names.push(stack.item.name.clone())
    }
    names
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // stack.item.name is String (NOT Copy) — clone IS expected
    // (either from explicit .clone() in source or auto-clone)
    assert!(
        generated.contains(".clone()"),
        "String field should still use .clone() when accessed through borrowed iterator.\nGenerated:\n{}",
        generated
    );
}
