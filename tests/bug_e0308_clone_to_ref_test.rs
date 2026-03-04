/// TDD Test: Over-cloning when method expects &String
///
/// Problem: ingredient.item_id.clone() passed to method expecting &String
/// Error: expected `&String`, found `String`
/// Root Cause: Compiler inserts .clone() but method signature wants &String
///
/// Solution: When passing struct.field to method expecting &String (owned field),
///           pass &struct.field instead of struct.field.clone()
use std::fs;
use std::process::Command;

#[test]
fn test_struct_field_to_ref_string_method() {
    let source = r#"
struct Item {
    id: string
}

struct Inventory {}

impl Inventory {
    pub fn has_item(self, id: string) -> bool {
        false
    }
}

fn check(inv: Inventory, item: Item) -> bool {
    inv.has_item(item.id)
}

fn main() {
    let inv = Inventory {}
    let item = Item { id: "sword" }
    let has = check(inv, item)
    println("{}", has)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let _output = Command::new("wj")
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

    // Verify no unnecessary .clone() when passing owned field to &String method
    assert!(
        generated.contains("has_item(&item.id)") || generated.contains("has_item(item.id)"),
        "Should pass owned field as reference, not clone"
    );

    fs::remove_dir_all(&test_dir).ok();
}
