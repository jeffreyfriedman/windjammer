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

fn compile_wj_source(source: &str) -> String {
    compile_wj_source_named(source, "default")
}

fn compile_wj_source_named(source: &str, name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("wj_vec_index_ctx_{}", name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
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

fn compile_wj_to_rust_and_check(source: &str) -> (String, bool) {
    let dir = std::env::temp_dir().join("wj_vec_index_context_check");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    // Find generated main.rs
    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    // Try to compile the generated Rust
    let bin_output = dir.join("test_bin");
    let rustc = Command::new("rustc")
        .args(&["--edition", "2021", "-o", bin_output.to_str().unwrap()])
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
    /// Bug: `self.items[i] = value` generates `self.items[i as usize].clone() = value`
    /// The .clone() on the LEFT side of assignment is always wrong.
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
    /// Bug: `&self.items[i]` generates `&self.items[i as usize].clone()`
    /// Taking a reference to a clone is pointless and wrong (reference to temporary).
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
    /// Bug: `&mut self.items[i]` generates `&mut self.items[i as usize].clone()`
    /// Can't take &mut of a temporary.
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
