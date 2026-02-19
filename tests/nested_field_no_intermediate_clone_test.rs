/// TDD Test: No .clone() on intermediate objects when reading nested Copy fields
///
/// Bug: When accessing a nested Copy field through a borrowed variable:
///   `stack.item.stats.armor` (where armor is f32)
/// the compiler generates `stack.item.clone().stats.armor`, cloning the entire
/// intermediate `Item` struct just to read a nested f32.
///
/// Root Cause: The borrowed_iterator_vars clone logic adds .clone() to
/// non-Copy fields accessed through borrowed variables, but doesn't check
/// `in_field_access_object`. When `stack.item` is the object of a parent
/// FieldAccess (`.stats`), no clone is needed because Rust auto-derefs
/// through references for nested field access.
///
/// Expected: `stack.item.stats.armor` (no .clone() on intermediate object)
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
fn test_no_clone_on_intermediate_for_nested_copy_field() {
    // Exact pattern from windjammer-game rpg/inventory.wj:
    // if let Some(stack) = &self.head { total.armor + stack.item.stats.armor }
    // stack.item is Item (non-Copy), but stats.armor is f32 (Copy).
    // No clone needed on the intermediate — Rust auto-derefs through &.
    let source = r#"
pub struct Stats {
    pub armor: f32,
    pub damage: f32,
    pub health: f32,
}

pub struct Item {
    pub name: string,
    pub stats: Stats,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub struct Equipment {
    pub head: Option<ItemStack>,
    pub body: Option<ItemStack>,
}

impl Equipment {
    pub fn total_armor(self) -> f32 {
        let mut total = 0.0
        if let Some(stack) = &self.head {
            total = total + stack.item.stats.armor
        }
        if let Some(stack) = &self.body {
            total = total + stack.item.stats.armor
        }
        total
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // stack.item.clone().stats.armor is wasteful — should be stack.item.stats.armor
    assert!(
        !generated.contains("stack.item.clone().stats"),
        "Should not clone intermediate 'item' just to read nested Copy field 'stats.armor'.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_intermediate_via_borrowed_iter() {
    // Same pattern through a for-loop borrowed iterator
    let source = r#"
pub struct Stats {
    pub armor: f32,
    pub damage: f32,
}

pub struct Item {
    pub name: string,
    pub stats: Stats,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub fn sum_armor(stacks: Vec<ItemStack>) -> f32 {
    let mut total = 0.0
    for stack in &stacks {
        total = total + stack.item.stats.armor
    }
    total
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // Should not clone the intermediate item for nested Copy field access
    assert!(
        !generated.contains("stack.item.clone().stats"),
        "Should not clone intermediate 'item' in borrowed iter for nested Copy field.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_clone_on_intermediate_when_consuming_non_copy_field() {
    // When the FINAL field is non-Copy (String), a clone IS needed somewhere
    let source = r#"
pub struct Item {
    pub name: string,
    pub weight: f32,
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

    // When the final field is String (non-Copy), clone IS needed
    // (either via explicit .clone() in source or auto-clone)
    assert!(
        generated.contains(".clone()"),
        "Should still use .clone() when accessing non-Copy field through borrowed iter.\nGenerated:\n{}",
        generated
    );
}
