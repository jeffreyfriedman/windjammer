#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_and_check(test_name: &str, wj_source: &str, expected_patterns: &[&str]) {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("test_{}", test_name));

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let test_file = test_dir.join(format!("{}.wj", test_name));
    fs::write(&test_file, wj_source).unwrap();

    let output = Command::new(test_utils::wj_binary())
        .current_dir(&test_dir)
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.join("build").join(format!("{}.rs", test_name));
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_else(|_| {
        panic!("Generated Rust file not found at {:?}", rust_file);
    });
    println!("Generated Rust:\n{}", rust_code);

    for pattern in expected_patterns {
        assert!(
            rust_code.contains(pattern),
            "Expected pattern '{}' not found in generated code:\n{}",
            pattern,
            rust_code
        );
    }

    let compile_output = Command::new("rustc")
        .current_dir(test_dir.join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg(format!("{}.rs", test_name))
        .output()
        .expect("Failed to run rustc");

    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    assert!(
        compile_output.status.success(),
        "Generated code failed to compile.\nRustc errors:\n{}\nGenerated code:\n{}",
        compile_stderr,
        rust_code
    );

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_tuple_key_auto_borrow() {
    compile_and_check(
        "hashmap_tuple_key",
        r#"
use std::collections::HashMap;

struct PathFinder {
    came_from: HashMap<(i32, i32), (i32, i32)>
}

impl PathFinder {
    fn trace_path(self, target: (i32, i32)) -> Vec<(i32, i32)> {
        let mut path = Vec::new()
        let mut current = target
        loop {
            match self.came_from.get(current) {
                Some(prev) => {
                    path.push(current)
                    current = prev
                }
                None => {
                    break
                }
            }
        }
        path
    }
}

fn main() {}
"#,
        &["came_from.get(&current)"],
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_usize_key_auto_borrow() {
    compile_and_check(
        "hashmap_usize_key",
        r#"
use std::collections::HashMap;

struct SaveManager {
    slots: HashMap<usize, string>
}

impl SaveManager {
    fn get_save(self, slot: usize) -> Option<string> {
        match self.slots.get(slot) {
            Some(data) => Some(data),
            None => None
        }
    }

    fn delete_save(self, slot: usize) {
        self.slots.remove(slot)
    }
}

fn main() {}
"#,
        &["slots.get(&slot)", "slots.remove(&slot)"],
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_cast_key_auto_borrow() {
    compile_and_check(
        "hashmap_cast_key",
        r#"
use std::collections::HashMap;

struct Panel {
    names: HashMap<i64, string>
}

impl Panel {
    fn get_name(self, entity_id: i32) -> string {
        match self.names.get(entity_id as i64) {
            Some(name) => name,
            None => "Unknown".to_string()
        }
    }

    fn remove_entity(self, entity_id: i32) {
        self.names.remove(entity_id as i64)
    }
}

fn main() {}
"#,
        &[
            "names.get(&(entity_id as i64))",
            "names.remove(&(entity_id as i64))",
        ],
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_remove_no_ref() {
    compile_and_check(
        "vec_remove_no_ref",
        r#"
fn remove_at(items: Vec<string>, idx: usize) {
    items.remove(idx)
}

fn main() {
    let mut items = Vec::new()
    items.push("hello".to_string())
    remove_at(items, 0)
}
"#,
        &["items.remove(idx)"],
    );
}
