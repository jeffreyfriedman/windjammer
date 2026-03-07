/// TDD Test: HashMap on self.field with borrowed String parameter
///
/// Problem: self.name_index.contains_key(&name) where name: &String
/// Error: String: Borrow<&String> not satisfied
/// Root cause: &(&String) = &&String, HashMap expects &str
///
/// This reproduces the exact pattern from scene_graph_state.wj
use std::fs;
use std::process::Command;

#[test]
fn test_self_field_hashmap_contains_key_borrowed_param() {
    let source = r#"
use std::collections::HashMap

struct Registry {
    name_index: HashMap<string, u64>
}

impl Registry {
    pub fn find_by_name(name: string) -> u64 {
        if self.name_index.contains_key(&name) {
            self.name_index.get(&name).unwrap().clone()
        } else {
            0
        }
    }
}

fn main() {
    let mut reg = Registry { name_index: HashMap::new() }
    reg.name_index.insert("test".to_string(), 42)
    let id = reg.find_by_name("test")
    println("{}", id)
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

    // WINDJAMMER DESIGN: String params infer to &str (not &String!)
    // - Read-only string param → &str (idiomatic Rust)
    // - User wrote `&name` → we generate `contains_key(name)` (already &str, no extra &)
    assert!(
        generated.contains("name: &str"),
        "Should generate &str parameter. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("contains_key(name)"),
        "Should pass name directly (already &str). Got:\n{}",
        generated
    );

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

    fs::remove_dir_all(&test_dir).ok();
}
