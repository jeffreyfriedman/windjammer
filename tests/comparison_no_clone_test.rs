/// TDD Test: No .clone() on borrowed fields used in comparisons
///
/// Bug: `recipe.name.clone() == target` when `recipe` is from a borrowed iterator.
/// Comparisons work on references in Rust (&String == &String via PartialEq),
/// so cloning is unnecessary noise and wasted allocation.
///
/// Root Cause: The borrowed iterator clone logic fires on field accesses without
/// knowing the expression is used in a comparison context.
///
/// Fix: Set `suppress_borrowed_clone = true` when generating operands of comparison
/// operators (==, !=, <, >, <=, >=).
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
fn test_no_clone_on_string_field_in_equality() {
    // Pattern from windjammer-game: recipe.name == target in a for loop
    let source = r#"
pub struct Recipe {
    pub name: string,
    pub cost: i32,
}

pub fn find_recipe(recipes: Vec<Recipe>, target: string) -> i32 {
    for recipe in &recipes {
        if recipe.name == target {
            return recipe.cost
        }
    }
    return 0
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // recipe.name should NOT be cloned just for comparison
    assert!(
        !generated.contains("recipe.name.clone() =="),
        "Should not clone String field for == comparison.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_string_field_in_not_equal() {
    // Pattern from windjammer-game: member.id != npc_id
    let source = r#"
pub struct Member {
    pub npc_id: string,
    pub role: string,
}

pub fn find_other(members: Vec<Member>, npc_id: string) -> i32 {
    let mut count = 0
    for member in &members {
        if member.npc_id != npc_id {
            count = count + 1
        }
    }
    count
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        !generated.contains(".clone() !="),
        "Should not clone String field for != comparison.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_clone_still_works_when_consuming_string() {
    // When a borrowed field's value is actually CONSUMED (not just compared),
    // .clone() IS still needed
    let source = r#"
pub struct Item {
    pub name: string,
}

pub fn collect_names(items: Vec<Item>) -> Vec<string> {
    let mut names: Vec<string> = Vec::new()
    for item in &items {
        names.push(item.name.clone())
    }
    names
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // When pushing to Vec (consuming), clone IS needed
    assert!(
        generated.contains(".clone()"),
        "Clone should still be used when consuming borrowed String field.\nGenerated:\n{}",
        generated
    );
}
