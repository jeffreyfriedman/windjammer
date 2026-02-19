/// TDD Test: No .clone() on intermediate Vec when using nested indexing
///
/// Bug: `self.tiles[row][col]` generates `self.tiles[row as usize].clone()[col as usize]`
/// which clones the ENTIRE inner Vec just to access one element.
/// In Rust, `vec[i][j]` auto-derefs through the reference returned by the first index.
///
/// Root Cause: The auto-clone for Vec indexing fired on the inner Index expression
/// because `in_field_access_object` wasn't set when generating the object of an Index.
///
/// Fix: Set `in_field_access_object = true` before generating the object of an Index,
/// same as we do for FieldAccess objects.
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
fn test_no_clone_on_2d_vec_nested_index() {
    // Pattern from windjammer-game world/tilemap.wj:
    // self.tiles[row][col] should NOT clone the entire row Vec
    let source = r#"
pub struct Grid {
    pub cells: Vec<Vec<i32>>,
}

impl Grid {
    pub fn get(self, row: i32, col: i32) -> i32 {
        self.cells[row][col]
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // Should NOT clone the inner Vec: cells[row].clone()[col] is WRONG
    assert!(
        !generated.contains(".clone()["),
        "Should not clone intermediate Vec in nested indexing.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_2d_bool_nested_index() {
    // Pattern from windjammer-game pathfinding/grid.wj:
    // self.walkable[x][y] — bool is Copy, no clone needed at all
    let source = r#"
pub struct PathGrid {
    pub walkable: Vec<Vec<bool>>,
}

impl PathGrid {
    pub fn is_walkable(self, x: i32, y: i32) -> bool {
        self.walkable[x][y]
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        !generated.contains(".clone()["),
        "Should not clone intermediate Vec for bool (Copy) nested index.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains(".clone()"),
        "Bool is Copy — no clone needed at all.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_2d_vec_non_copy_final_element_clone_ok() {
    // When the final element is non-Copy and consumed, .clone() on the element IS ok
    // But the intermediate Vec should NEVER be cloned
    let source = r#"
pub struct Grid {
    pub names: Vec<Vec<string>>,
}

impl Grid {
    pub fn get_name(self, row: i32, col: i32) -> string {
        self.names[row][col].clone()
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // The final element clone is OK (String is not Copy)
    // But the intermediate Vec clone is NOT OK
    assert!(
        !generated.contains("].clone()["),
        "Should not clone intermediate Vec even when final element needs clone.\nGenerated:\n{}",
        generated
    );
}
