/// TDD Test: HashMap String key methods on self.field - E0277 fix
/// 
/// Problem: self.name_to_id.contains_key(&name) where name: &String causes E0277
/// Error: the trait bound `String: Borrow<&String>` is not satisfied
/// Root Cause: HashMap is a field, not a parameter, so current fix doesn't apply
/// 
/// Solution: Extend HashMap fix to work with field access patterns

use std::fs;
use std::process::Command;

#[test]
fn test_hashmap_field_contains_key() {
    let source = r#"
use std::collections::HashMap

struct Manager {
    name_to_id: HashMap<string, int>
}

impl Manager {
    pub fn has_name(self, name: string) -> bool {
        self.name_to_id.contains_key(&name)
    }
}

fn main() {
    let mut mgr = Manager { name_to_id: HashMap::new() }
    mgr.name_to_id.insert("test", 42)
    let exists = mgr.has_name("test")
    assert_eq(exists, true)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!("wj_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
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
    let generated = fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
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
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
    }
    
    // Verify HashMap method doesn't add double &
    assert!(
        generated.contains("contains_key(name)") || 
        generated.contains("contains_key(&**name)"),
        "HashMap::contains_key on self.field should handle &String correctly"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}
