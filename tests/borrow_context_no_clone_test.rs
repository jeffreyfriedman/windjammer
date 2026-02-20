/// TDD Test: No .clone() on borrowed fields when used in borrow context (&expr)
///
/// Bug: `for ingredient in &recipe.ingredients.clone()` clones the entire Vec
/// just to iterate by reference. The `&` means we only need a reference, so
/// cloning is wasteful (O(n) allocation).
///
/// Root Cause: The borrowed iterator clone logic didn't check `in_borrow_context`.
/// When generating `&recipe.ingredients`, the `&` sets `in_borrow_context = true`,
/// but the FieldAccess handler didn't check this flag.
///
/// Fix: Added `!self.in_borrow_context` to the borrowed iterator clone condition.
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
fn test_no_clone_on_vec_field_in_for_loop_borrow() {
    // Pattern from windjammer-game rpg/crafting.wj:
    // for ingredient in &recipe.ingredients { ... }
    // recipe is borrowed — &recipe.ingredients is sufficient, no .clone() needed
    let source = r#"
pub struct Ingredient {
    pub name: string,
    pub amount: i32,
}

pub struct Recipe {
    pub ingredients: Vec<Ingredient>,
}

pub fn count_ingredients(recipes: Vec<Recipe>) -> i32 {
    let mut total = 0
    for recipe in &recipes {
        for ingredient in &recipe.ingredients {
            total = total + ingredient.amount
        }
    }
    total
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // Should NOT clone ingredients Vec just to iterate
    assert!(
        !generated.contains(".ingredients.clone()"),
        "Should not clone Vec field when iterating by reference.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_field_passed_by_ref() {
    // When passing a borrowed field by reference, no clone needed
    let source = r#"
pub struct Data {
    pub items: Vec<i32>,
}

pub fn process(data: Vec<Data>) -> i32 {
    let mut total = 0
    for d in &data {
        total = total + d.items.len() as i32
    }
    total
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // .len() takes &self, so no clone needed on items
    assert!(
        !generated.contains("d.items.clone()"),
        "Should not clone Vec field when calling .len() on it.\nGenerated:\n{}",
        generated
    );
}
