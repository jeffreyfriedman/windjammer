/// TDD Test: Vec::get(index) must NOT add & to index argument
///
/// Bug: The compiler generates `self.points.get(&i)` instead of `self.points.get(i)`
/// when calling .get() on a Vec. Vec::get takes `usize` by value, not by reference.
/// HashMap::get takes `&K` by reference, so the heuristic must distinguish between them.
///
/// Root cause: In method_call_analyzer.rs, needs_stdlib_ref() treats all .get() calls
/// as HashMap::get(&K), but Vec::get(usize) takes index by value.
///
/// Discovered via dogfooding: windjammer-ui/curve_editor.wj

use std::process::Command;

fn compile_wj_source_named(source: &str, name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("wj_vec_get_ref_{}", name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
            "--library",
        ])
        .output()
        .expect("Failed to run wj compiler");

    // Find the generated .rs file
    let mut rs_content = String::new();
    for entry in std::fs::read_dir(&dir)
        .unwrap()
        .chain(std::fs::read_dir(dir.join("src")).into_iter().flatten())
    {
        if let Ok(entry) = entry {
            if entry.path().extension().map_or(false, |e| e == "rs") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    rs_content.push_str(&content);
                    rs_content.push('\n');
                }
            }
        }
    }

    rs_content
}

#[test]
fn test_vec_get_no_ref_on_index() {
    // Vec::get(index) takes usize by value, NOT by reference
    // The compiler should generate .get(i) not .get(&i)
    let source = r#"
pub struct Point {
    pub x: f32,
    pub y: f32,
}

pub struct Path {
    pub points: Vec<Point>,
}

impl Path {
    pub fn get_point_x(&self, index: i32) -> f32 {
        let p = self.points.get(index as usize)
        match p {
            Some(point) => point.x,
            None => 0.0,
        }
    }
}
"#;

    let generated = compile_wj_source_named(source, "vec_get_no_ref");
    println!("Generated Rust:\n{}", generated);

    // Must NOT contain .get(&
    assert!(
        !generated.contains(".get(&"),
        "Vec::get should take index by value, not by reference.\nGenerated: {}",
        generated
    );

    // Must contain .get( without &
    assert!(
        generated.contains(".get("),
        "Should contain .get() call.\nGenerated: {}",
        generated
    );
}

#[test]
fn test_vec_get_with_loop_index() {
    // Common pattern: for i in 0..vec.len() { vec.get(i) }
    // The 'i' variable should NOT get & added
    let source = r#"
pub struct Item {
    pub name: string,
    pub value: i32,
}

pub struct Inventory {
    pub items: Vec<Item>,
}

impl Inventory {
    pub fn total_value(&self) -> i32 {
        let mut total = 0
        for i in 0..self.items.len() {
            let item = self.items.get(i)
            match item {
                Some(it) => {
                    total = total + it.value
                },
                None => {},
            }
        }
        total
    }
}
"#;

    let generated = compile_wj_source_named(source, "vec_get_loop");
    println!("Generated Rust:\n{}", generated);

    // Must NOT contain .get(&i) or .get(&
    assert!(
        !generated.contains(".get(&"),
        "Vec::get in loop should not add & to index.\nGenerated: {}",
        generated
    );
}

#[test]
fn test_vec_get_mut_no_ref_on_index() {
    // Vec::get_mut(index) also takes usize by value
    let source = r#"
pub struct Widget {
    pub x: f32,
    pub y: f32,
}

pub struct Layout {
    pub widgets: Vec<Widget>,
}

impl Layout {
    pub fn move_widget(&mut self, index: i32, dx: f32, dy: f32) {
        let w = self.widgets.get_mut(index as usize)
        match w {
            Some(widget) => {
                widget.x = widget.x + dx
                widget.y = widget.y + dy
            },
            None => {},
        }
    }
}
"#;

    let generated = compile_wj_source_named(source, "vec_get_mut_no_ref");
    println!("Generated Rust:\n{}", generated);

    // Must NOT contain .get_mut(&
    assert!(
        !generated.contains(".get_mut(&"),
        "Vec::get_mut should take index by value, not by reference.\nGenerated: {}",
        generated
    );
}
