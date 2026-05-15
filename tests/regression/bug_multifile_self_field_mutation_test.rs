#![cfg(not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
)))]

/// TDD Test: Self field mutation detection across multiple files
///
/// Bug: When structs are in SEPARATE files and compiled together with `--library`,
/// calling `self.field.mutating_method()` fails to infer `&mut self` for the
/// containing method.
///
/// Examples from windjammer-game:
/// - World::remove_transform calls self.transforms.remove(entity) → needs &mut self
/// - World::spawn calls self.allocator.allocate() → needs &mut self  
/// - NpcBehavior::update_patrol calls self.patrol.update_wait(dt) → needs &mut self
///
/// Works in single file (both structs together), breaks in multi-file compilation.
#[path = "../common/test_utils.rs"]
mod test_utils;

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile_windjammer_directory(
    files: &[(&str, &str)],
) -> Result<std::collections::HashMap<String, String>, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).expect("Failed to create src dir");
    let out_dir = temp_dir.path().join("build");

    for (name, content) in files {
        let file_path = src_dir.join(name);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create subdirs");
        }
        std::fs::write(&file_path, content).expect("Failed to write source file");
    }

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));

    let output = Command::new(&wj_binary)
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(format!(
            "Windjammer compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let mut results = std::collections::HashMap::new();
    for (name, _) in files {
        let rs_name = name.replace(".wj", ".rs");
        let generated_file = out_dir.join(&rs_name);
        if generated_file.exists() {
            let content = std::fs::read_to_string(&generated_file)
                .unwrap_or_else(|_| format!("Failed to read {}", rs_name));
            results.insert(rs_name, content);
        }
    }
    Ok(results)
}

#[test]
fn test_single_file_self_field_stdlib_remove_infers_mut() {
    let code = r#"
use std::collections::HashMap

pub struct Transform {
    pub x: f32,
    pub y: f32,
}

pub struct World {
    pub transforms: HashMap<i32, Transform>,
}

impl World {
    pub fn remove_transform(self, entity: i32) -> Option<Transform> {
        return self.transforms.remove(entity)
    }
}

fn main() {}
"#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");
    assert!(
        generated.contains("fn remove_transform(&mut self"),
        "remove_transform should infer &mut self because self.transforms.remove() mutates.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_multifile_self_field_stdlib_remove_infers_mut() {
    let files = vec![
        (
            "world.wj",
            r#"
use std::collections::HashMap

pub struct Transform {
    pub x: f32,
    pub y: f32,
}

pub struct World {
    pub transforms: HashMap<i32, Transform>,
}

impl World {
    pub fn remove_transform(self, entity: i32) -> Option<Transform> {
        return self.transforms.remove(entity)
    }

    pub fn get_transform(self, entity: i32) -> Option<Transform> {
        return self.transforms.get(entity)
    }
}
"#,
        ),
        (
            "other.wj",
            r#"
pub struct OtherSystem {
    pub name: String,
}

impl OtherSystem {
    pub fn get_name(self) -> String {
        return self.name.clone()
    }
}
"#,
        ),
    ];

    let results = compile_windjammer_directory(&files).expect("Compilation should succeed");
    let world_rs = results.get("world.rs").expect("world.rs should exist");
    assert!(
        world_rs.contains("fn remove_transform(&mut self"),
        "remove_transform should infer &mut self in multi-file mode.\nGenerated world.rs:\n{}",
        world_rs
    );
}

#[test]
fn test_multifile_self_field_user_method_infers_mut() {
    let files = vec![
        (
            "allocator.wj",
            r#"
pub struct IdAllocator {
    pub next_id: i32,
}

impl IdAllocator {
    pub fn allocate(self) -> i32 {
        let id = self.next_id
        self.next_id = self.next_id + 1
        return id
    }
}
"#,
        ),
        (
            "world.wj",
            r#"
use crate::allocator::IdAllocator

pub struct World {
    pub allocator: IdAllocator,
}

impl World {
    pub fn spawn(self) -> i32 {
        self.allocator.allocate()
    }
}
"#,
        ),
    ];

    let results = compile_windjammer_directory(&files).expect("Compilation should succeed");
    let world_rs = results.get("world.rs").expect("world.rs should exist");
    assert!(
        world_rs.contains("fn spawn(&mut self"),
        "spawn should infer &mut self because self.allocator.allocate() mutates allocator.\nGenerated world.rs:\n{}",
        world_rs
    );
}

#[test]
fn test_multifile_self_field_get_mut_infers_mut() {
    let files = vec![
        (
            "world.wj",
            r#"
use std::collections::HashMap

pub struct Transform {
    pub x: f32,
}

pub struct World {
    pub transforms: HashMap<i32, Transform>,
}

impl World {
    pub fn get_transform_mut(self, entity: i32) -> Option<Transform> {
        self.transforms.get_mut(entity)
    }
}
"#,
        ),
        (
            "helper.wj",
            r#"
pub fn helper() -> i32 {
    return 42
}
"#,
        ),
    ];

    let results = compile_windjammer_directory(&files).expect("Compilation should succeed");
    let world_rs = results.get("world.rs").expect("world.rs should exist");
    assert!(
        world_rs.contains("fn get_transform_mut(&mut self"),
        "get_transform_mut should infer &mut self because self.transforms.get_mut() mutates.\nGenerated world.rs:\n{}",
        world_rs
    );
}

/// Regression test: depth counter leak in expression_mutates_self_fields
///
/// Bug: Early `return true` inside expression_mutates_self_fields exited
/// without decrementing the thread-local depth counter (DEPTH_EM). After
/// enough successful mutation detections, the counter exceeded 1000 and
/// all subsequent calls short-circuited to `false`.
///
/// This test creates many files with mutating methods to exercise the
/// counter extensively, then checks that the LAST file still gets correct
/// &mut self inference.
#[test]
fn test_depth_counter_leak_many_files_still_infer_mut() {
    let mut files: Vec<(&str, &str)> = Vec::new();

    // Create many files that each have methods calling self.field.push() etc.
    // These successful mutation detections would leak depth counter increments.
    static FILE_A: &str = r#"
pub struct CollectionA {
    pub items: Vec<i32>,
}

impl CollectionA {
    pub fn add(self, item: i32) {
        self.items.push(item)
    }
    pub fn remove_last(self) -> Option<i32> {
        return self.items.pop()
    }
    pub fn clear_all(self) {
        self.items.clear()
    }
}
"#;

    static FILE_B: &str = r#"
pub struct CollectionB {
    pub data: Vec<f32>,
}

impl CollectionB {
    pub fn insert_at(self, idx: i32, val: f32) {
        self.data.insert(idx, val)
    }
    pub fn remove_at(self, idx: i32) -> f32 {
        return self.data.remove(idx)
    }
    pub fn sort_data(self) {
        self.data.sort()
    }
}
"#;

    static FILE_C: &str = r#"
pub struct CollectionC {
    pub entries: Vec<i32>,
}

impl CollectionC {
    pub fn extend_from(self, other: Vec<i32>) {
        self.entries.extend(other)
    }
    pub fn truncate_to(self, len: i32) {
        self.entries.truncate(len)
    }
    pub fn reverse_order(self) {
        self.entries.reverse()
    }
}
"#;

    static FILE_D: &str = r#"
pub struct CollectionD {
    pub values: Vec<i32>,
}

impl CollectionD {
    pub fn push_val(self, v: i32) {
        self.values.push(v)
    }
    pub fn pop_val(self) -> Option<i32> {
        return self.values.pop()
    }
    pub fn drain_all(self) {
        self.values.clear()
    }
}
"#;

    files.push(("collection_a.wj", FILE_A));
    files.push(("collection_b.wj", FILE_B));
    files.push(("collection_c.wj", FILE_C));
    files.push(("collection_d.wj", FILE_D));

    // The critical file - analyzed AFTER the others have exercised mutation detection.
    // If the depth counter leaked, this file's analysis would fail.
    static FINAL_FILE: &str = r#"
use std::collections::HashMap

pub struct Entity {
    pub id: i32,
}

pub struct Transform {
    pub x: f32,
    pub y: f32,
}

pub struct FinalWorld {
    pub transforms: HashMap<i32, Transform>,
    pub entities: Vec<Entity>,
}

impl FinalWorld {
    pub fn remove_transform(self, entity_id: i32) -> Option<Transform> {
        return self.transforms.remove(entity_id)
    }

    pub fn add_entity(self, entity: Entity) {
        self.entities.push(entity)
    }
}
"#;

    files.push(("final_world.wj", FINAL_FILE));

    let results = compile_windjammer_directory(&files).expect("Compilation should succeed");
    let final_rs = results
        .get("final_world.rs")
        .expect("final_world.rs should exist");

    assert!(
        final_rs.contains("fn remove_transform(&mut self"),
        "remove_transform must infer &mut self even after many prior mutation detections.\nGenerated final_world.rs:\n{}",
        final_rs
    );
    assert!(
        final_rs.contains("fn add_entity(&mut self"),
        "add_entity must infer &mut self even after many prior mutation detections.\nGenerated final_world.rs:\n{}",
        final_rs
    );
}
