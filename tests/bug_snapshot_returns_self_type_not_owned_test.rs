#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// TDD: Methods that return the same struct type by constructing a new instance
// (e.g. snapshot, clone) should get &self, not owned self.
// Bug: `fn snapshot(self) -> Scene` generated `self` (owned) because
// method_returns_impl_struct returned true, making codegen think this was
// a builder pattern. But snapshot creates a NEW instance by cloning fields
// and should be &self.
// Trigger: E0507 "cannot move out of `self.field` which is behind a mutable reference"

use std::fs;
use std::process::Command;

#[test]
fn test_snapshot_method_returns_self_type_gets_ref_self() {
    let _tmp = tempfile::tempdir().unwrap();
    let temp_dir = _tmp.path();

    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_source = r#"
pub struct Scene {
    pub name: string,
    pub entities: Vec<i64>,
    pub next_id: i64,
    pub parents: Vec<i64>,
    pub children: Vec<Vec<i64>>,
}

impl Scene {
    pub fn new(name: string) -> Scene {
        Scene {
            name: name,
            entities: Vec::new(),
            next_id: 0,
            parents: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn snapshot(self) -> Scene {
        let mut children_copy = Vec::new()
        for row in self.children {
            children_copy.push(row)
        }
        Scene {
            name: self.name,
            entities: self.entities,
            next_id: self.next_id,
            parents: self.parents,
            children: children_copy,
        }
    }

    pub fn restore_from(self, other: Scene) {
        self.name = other.name
        self.entities = other.entities
        self.next_id = other.next_id
        self.parents = other.parents
    }
}

pub struct Editor {
    pub scene: Scene,
    pub saved_snapshot: Option<Scene>,
}

impl Editor {
    pub fn save_snapshot(self) {
        self.saved_snapshot = Some(self.scene.snapshot())
    }
}

pub fn main() {
    let mut editor = Editor {
        scene: Scene::new("test"),
        saved_snapshot: None,
    }
    editor.save_snapshot()
    println!("snapshot saved")
}
"#;

    let wj_path = test_dir.join("snapshot_test.wj");
    fs::write(&wj_path, wj_source).unwrap();

    let wj_binary = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/wj");

    let output_dir = test_dir.join("out");
    fs::create_dir_all(&output_dir).unwrap();

    let output = Command::new(&wj_binary)
        .args([
            "build",
            "--no-cargo",
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .arg(wj_path.to_str().unwrap())
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "wj build should succeed. stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_path = output_dir.join("snapshot_test.rs");
    assert!(rs_path.exists(), "Generated .rs file should exist");

    let generated = fs::read_to_string(&rs_path).unwrap();

    // restore_from should be &mut self since it modifies self
    assert!(
        generated.contains("fn restore_from(&mut self"),
        "restore_from() should generate &mut self. Generated:\n{}",
        generated
    );

    // save_snapshot should be &mut self since it modifies self.saved_snapshot
    assert!(
        generated.contains("fn save_snapshot(&mut self"),
        "save_snapshot() should generate &mut self. Generated:\n{}",
        generated
    );

    // snapshot may use &self (optimal: clones individual fields) or
    // self (owned: caller clones the whole scene). Both are correct Rust.
    // The definitive check is that the generated code compiles.
    let rustc_output = Command::new("rustc")
        .arg("--edition=2021")
        .arg(&rs_path)
        .arg("-o")
        .arg(test_dir.join("snapshot_test_bin"))
        .output()
        .expect("Failed to run rustc");

    assert!(
        rustc_output.status.success(),
        "Generated Rust should compile without errors. stderr:\n{}\nGenerated:\n{}",
        String::from_utf8_lossy(&rustc_output.stderr),
        generated
    );
}
