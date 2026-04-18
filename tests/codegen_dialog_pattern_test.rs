/// TDD test for dialog.wj specific pattern
/// 
/// Bug: When method with `self` calls another method with String parameter,
/// and the String comes from a match arm that consumes self, we get E0308.

use std::fs;
use std::process::Command;

#[test]
fn test_dialog_pattern_self_consumption() {
    // This test replicates the pattern from dialog.wj
    // Should compile successfully with automatic String → &str conversion
    let source = r#"
enum Condition {
    HasItem(string, i32),
    HasGold(i32),
}

struct Inventory {
    items: Vec<(string, i32)>,
}

impl Inventory {
    pub fn new() -> Inventory {
        Inventory { items: Vec::new() }
    }
    
    pub fn has_item(self, item_id: string, min_qty: i32) -> bool {
        for (id, qty) in self.items {
            if id == item_id {
                return qty >= min_qty
            }
        }
        false
    }
}

struct GameState {
    inventory: Inventory,
}

impl Condition {
    pub fn evaluate(self, game_state: GameState) -> bool {
        match self {
            Condition::HasItem(item_id, qty) => {
                game_state.inventory.has_item(item_id, qty)
            },
            Condition::HasGold(_) => false,
        }
    }
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_dialog_pattern_{}",
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

    // Check what was generated for has_item
    if generated.contains("pub fn has_item(&self") {
        println!("✓ has_item inferred as &self (read-only)");
    } else if generated.contains("pub fn has_item(self") {
        println!("✓ has_item inferred as self (owned)");
    }

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
        let stdout = String::from_utf8_lossy(&rustc_output.stdout);
        panic!(
            "Compilation failed:\nSTDERR:\n{}\nSTDOUT:\n{}\n\nGenerated code:\n{}",
            stderr, stdout, generated
        );
    }

    fs::remove_dir_all(&test_dir).ok();
}
