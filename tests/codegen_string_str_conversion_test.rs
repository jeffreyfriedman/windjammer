/// TDD test for automatic String → &str conversion at call sites
///
/// Bug: When owned String values (from match arms, returns, etc.) are passed
/// to functions expecting &str, the compiler generates type mismatch errors.
///
/// Example that should work:
/// ```windjammer
/// enum Condition {
///     HasItem(string),
/// }
///
/// fn check_item(item_id: string) -> bool {
///     has_item(item_id)  // item_id is owned String, has_item expects &str
/// }
///
/// fn has_item(id: string) -> bool {
///     id == "sword"
/// }
/// ```
///
/// Current behavior: E0308 type mismatch (String vs &str)
/// Expected behavior: Compiler automatically converts String → &str at call site
use std::fs;
use std::process::Command;

#[test]
fn test_string_to_str_conversion_in_match_arms() {
    let source = r#"
enum Cond {
    HasItem(string),
    HasGold(i32),
}

struct Inventory {
    items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item_id: string) -> bool {
        for item in self.items {
            if item == item_id {
                return true
            }
        }
        false
    }
}

fn check_condition(cond: Cond, inventory: Inventory) -> bool {
    match cond {
        Cond::HasItem(item_id) => {
            // item_id is owned String from match arm
            // has_item expects &str (inferred for read-only param)
            inventory.has_item(item_id)
        },
        Cond::HasGold(_) => false,
    }
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_string_conversion_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let _output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj compiler");

    let out_dir = test_dir.join("build");
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    // The generated code should compile with rustc
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test.rlib"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Compilation failed - String → &str conversion not working:\n{}\n\nGenerated code:\n{}",
            stderr, generated
        );
    }

    fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn test_string_literal_vs_owned_string() {
    let source = r#"
struct GameState {
    name: string,
}

impl GameState {
    pub fn get_flag(self, flag_name: string) -> bool {
        self.has_flag(flag_name)
    }
    
    pub fn has_flag(self, name: string) -> bool {
        self.name == name
    }
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_string_literal_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let _output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj compiler");

    let out_dir = test_dir.join("build");
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    // Should compile successfully
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test.rlib"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, generated
        );
    }

    fs::remove_dir_all(&test_dir).ok();
}
