/// TDD Test: Vec index auto-clone must be suppressed in certain contexts
///
/// Bug: The compiler adds .clone() to Vec<T>[i] for non-Copy types, but
/// this is incorrect in three contexts:
///
/// 1. Assignment target: `self.vec[i] = value` → must NOT clone (can't assign to .clone())
/// 2. Borrow context: `&self.vec[i]` → must NOT clone (want reference to original)
/// 3. Mutable borrow context: `&mut self.vec[i]` → must NOT clone (can't take &mut of temp)
///
/// Root cause: The Expression::Index handler's auto-clone fallback doesn't check
/// whether we're generating an assignment target or inside a reference expression.
///
/// Discovered via dogfooding: ecs/components.wj (ComponentArray<T>)
use std::process::Command;
use tempfile::tempdir;

fn compile_wj_source_named(source: &str, _name: &str) -> String {
    let dir = tempdir().expect("tempdir for compile_wj_source_named");

    let wj_file = dir.path().join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--library",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let mut rs_content = String::new();
    for search_dir in [&out, &dir.path().join("src")] {
        if let Ok(entries) = std::fs::read_dir(search_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "rs") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        rs_content.push_str(&content);
                        rs_content.push('\n');
                    }
                }
            }
        }
    }

    rs_content
}

fn compile_wj_to_rust_and_check(source: &str) -> (String, bool) {
    let dir = tempdir().expect("tempdir for compile_wj_to_rust_and_check");

    let wj_file = dir.path().join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = out.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        out.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
            "--edition",
            "2021",
            "-o",
        ])
        .arg(dir.path().join("verify.rmeta"))
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

#[test]
fn test_vec_index_no_clone_on_assignment_target() {
    // Bug: `self.items[i] = value` generates `self.items[i as usize].clone() = value`
    // The .clone() on the LEFT side of assignment is always wrong.
    let source = r#"
struct Item {
    name: string,
    count: i32,
}

struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: vec![] }
    }

    pub fn update_at(&mut self, index: i32, item: Item) {
        self.items[index as usize] = item
    }
}

fn main() {
    let mut c = Container::new()
    c.items.push(Item { name: "old".to_string(), count: 0 })
    c.update_at(0, Item { name: "new".to_string(), count: 1 })
    println("done")
}
"#;

    let (rust_code, compiles) = compile_wj_to_rust_and_check(source);

    // The assignment target must NOT have .clone()
    // Bad: self.items[index as usize].clone() = item
    // Good: self.items[index as usize] = item
    assert!(
        !rust_code.contains(".clone() = "),
        "Assignment target must NOT have .clone()!\nGenerated:\n{}",
        rust_code
    );

    assert!(
        compiles,
        "Generated Rust must compile!\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_vec_index_no_clone_on_borrow() {
    // Bug: `&self.items[i]` generates `&self.items[i as usize].clone()`
    // Taking a reference to a clone is pointless and wrong (reference to temporary).
    let source = r#"
struct Item {
    name: string,
    count: i32,
}

struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: vec![] }
    }

    pub fn get_ref(&self, index: i32) -> &Item {
        &self.items[index as usize]
    }
}

fn main() {
    let c = Container::new()
    println("done")
}
"#;

    let rust_code = compile_wj_source_named(source, "borrow");

    // Borrow context must NOT have .clone()
    // Bad: &self.items[index as usize].clone()
    // Good: &self.items[index as usize]
    assert!(
        !rust_code.contains(".clone()"),
        "Borrow of Vec index must NOT have .clone()!\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_vec_index_no_clone_on_mut_borrow() {
    // Bug: `&mut self.items[i]` generates `&mut self.items[i as usize].clone()`
    // Can't take &mut of a temporary.
    let source = r#"
struct Item {
    name: string,
    count: i32,
}

struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: vec![] }
    }

    pub fn get_mut_ref(&mut self, index: i32) -> &mut Item {
        &mut self.items[index as usize]
    }
}

fn main() {
    let mut c = Container::new()
    println("done")
}
"#;

    let rust_code = compile_wj_source_named(source, "mut_borrow");

    // Mutable borrow context must NOT have .clone()
    // Bad: &mut self.items[index as usize].clone()
    // Good: &mut self.items[index as usize]
    assert!(
        !rust_code.contains(".clone()"),
        "Mutable borrow of Vec index must NOT have .clone()!\nGenerated:\n{}",
        rust_code
    );
}
