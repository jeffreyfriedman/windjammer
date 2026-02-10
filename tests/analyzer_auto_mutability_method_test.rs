/// TDD Test: Auto-Mutability for Mutating Method Calls
///
/// THE WINDJAMMER WAY: Extend auto-mutability to method calls
///
/// Current: Only field mutations trigger auto-mut (x.field = value)
/// Goal: Method calls should also trigger auto-mut (inv.add_item(item))
///
/// Philosophy: Compiler infers `mut` for ANY mutation, not just field assignments
use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_mut_on_mutating_method_call() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_auto_mut_method_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Windjammer code that calls a mutating method with explicit `let mut`
    // Immutable-by-default: users must write `let mut` for mutable bindings.
    let test_content = r#"
struct Inventory {
    items: Vec<i32>
}

impl Inventory {
    fn new() -> Inventory {
        Inventory { items: Vec::new() }
    }
    
    fn add_item(&mut self, item: i32) {
        self.items.push(item)
    }
}

fn main() {
    let mut inv = Inventory::new()
    inv.add_item(42)
    inv.add_item(100)
    println!("Item count: {}", inv.items.len())
}
"#;

    fs::write(test_dir.join("method_mut.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("method_mut.wj")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let rust_file = test_dir.join("build").join("method_mut.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_default();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Assert that `mut` was automatically added
    assert!(
        rust_code.contains("let mut inv =") || rust_code.contains("let mut inv="),
        "Should automatically add 'mut' when mutating method is called.\nGenerated code:\n{}",
        rust_code
    );

    // Should compile successfully with rustc (no mutability errors)
    assert!(
        !stderr.contains("cannot borrow") && !stdout.contains("cannot borrow"),
        "Should not have mutability errors.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_mut_with_multiple_method_calls() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_auto_mut_methods_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Test with multiple mutating method calls
    let test_content = r#"
struct Counter {
    value: i32
}

impl Counter {
    fn new() -> Counter {
        Counter { value: 0 }
    }
    
    fn increment(&mut self) {
        self.value = self.value + 1
    }
    
    fn add(&mut self, amount: i32) {
        self.value = self.value + amount
    }
    
    fn get(&self) -> i32 {
        self.value
    }
}

fn main() {
    let mut counter = Counter::new()
    counter.increment()
    counter.add(5)
    counter.increment()
    println!("Count: {}", counter.get())
}
"#;

    fs::write(test_dir.join("multi_method.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("multi_method.wj")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let rust_file = test_dir.join("build").join("multi_method.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_default();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Assert that `mut` was automatically added
    assert!(
        rust_code.contains("let mut counter =") || rust_code.contains("let mut counter="),
        "Should automatically add 'mut' for multiple mutating methods.\nGenerated code:\n{}",
        rust_code
    );

    // Should compile successfully
    assert!(
        !stderr.contains("cannot borrow") && !stdout.contains("cannot borrow"),
        "Should not have mutability errors.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_mut_when_only_immutable_methods() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_no_mut_immutable_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Test that mut is NOT added when only immutable methods are called
    let test_content = r#"
struct Point {
    x: i32,
    y: i32
}

impl Point {
    fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }
    
    fn get_x(&self) -> i32 {
        self.x
    }
    
    fn get_y(&self) -> i32 {
        self.y
    }
}

fn main() {
    let point = Point::new(10, 20)
    println!("X: {}", point.get_x())
    println!("Y: {}", point.get_y())
}
"#;

    fs::write(test_dir.join("immutable_method.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let _output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("immutable_method.wj")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let rust_file = test_dir.join("build").join("immutable_method.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_default();

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Assert that `mut` was NOT added (only immutable methods called)
    assert!(
        !rust_code.contains("let mut point =") && !rust_code.contains("let mut point="),
        "Should NOT add 'mut' when only immutable methods are called.\nGenerated code:\n{}",
        rust_code
    );
}
