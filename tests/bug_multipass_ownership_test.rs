/// TDD Test: Multi-Pass Ownership Inference
///
/// Verifies that string parameters used only for comparison/passthrough
/// are correctly inferred as borrowed (&str) and that the generated Rust compiles.
use std::fs;
use std::process::Command;

#[test]
fn test_passthrough_borrowed_convergence() {
    let source = r#"
fn leaf_fn(id: string) -> bool {
    id == "test"
}

fn wrapper_fn(item_id: string) -> bool {
    leaf_fn(item_id)
}

fn main() {
    let id = "sword"
    let result = wrapper_fn(id)
}
"#;

    let temp_dir = std::env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let thread_id = std::thread::current().id();
    let test_id = format!("wj_test_{}_{:?}", timestamp, thread_id);
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let _output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, generated
        );
    }

    // String params used only for comparison should be inferred as &str
    assert!(
        generated.contains("fn leaf_fn(id: &str)")
            || generated.contains("fn leaf_fn(_id: &str)")
            || generated.contains("fn leaf_fn(id: &String)")
            || generated.contains("fn leaf_fn(_id: &String)"),
        "leaf_fn should have borrowed param. Generated:\n{}",
        generated
    );

    fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn test_method_passthrough_convergence() {
    let source = r#"
struct Inventory {
    items: Vec<string>
}

impl Inventory {
    fn has(id: string) -> bool {
        for item in &self.items {
            if item == id {
                return true
            }
        }
        false
    }
}

struct Merchant {
    inventory: Inventory
}

impl Merchant {
    fn has_item(item_id: string) -> bool {
        self.inventory.has(item_id)
    }
}

fn check(merchant: Merchant) -> bool {
    merchant.has_item("sword")
}

fn main() {}
"#;

    let temp_dir = std::env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let thread_id = std::thread::current().id();
    let test_id = format!("wj_test_{}_{:?}", timestamp, thread_id);
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let _output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, generated
        );
    }

    // Verify the method params are borrowed (either &str or &String)
    let has_borrowed_has = generated.contains("fn has(&self, id: &str)")
        || generated.contains("fn has(&self, id: &String)");
    assert!(
        has_borrowed_has,
        "Inventory::has should have borrowed param. Generated:\n{}",
        generated
    );

    fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn test_circular_dependency_convergence() {
    // Edge case: mutual recursion should converge
    let source = r#"
fn foo(x: string) -> bool {
    if x == "stop" {
        true
    } else {
        bar(x)
    }
}

fn bar(y: string) -> bool {
    if y == "stop" {
        false
    } else {
        foo(y)
    }
}

fn main() {
    let result = foo("test")
}
"#;

    let temp_dir = std::env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let thread_id = std::thread::current().id();
    let test_id = format!("wj_test_{}_{:?}", timestamp, thread_id);
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let _output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, generated
        );
    }

    // Both should converge to borrowed (either &str or &String)
    let foo_borrowed = generated.contains("fn foo(x: &str)")
        || generated.contains("fn foo(x: &String)")
        || generated.contains("fn foo(_x: &str)")
        || generated.contains("fn foo(_x: &String)");
    assert!(
        foo_borrowed,
        "foo should have borrowed param after convergence. Generated:\n{}",
        generated
    );
    let bar_borrowed = generated.contains("fn bar(y: &str)")
        || generated.contains("fn bar(y: &String)")
        || generated.contains("fn bar(_y: &str)")
        || generated.contains("fn bar(_y: &String)");
    assert!(
        bar_borrowed,
        "bar should have borrowed param after convergence. Generated:\n{}",
        generated
    );

    fs::remove_dir_all(&test_dir).ok();
}
