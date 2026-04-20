//! TDD: Read-only recursive methods must get `&self`, not `&mut self`.
//!
//! Bug: The recursion detector in `function_modifies_self_fields_recursive` treats
//! cycles as conservative `true` (assume mutation). But for read-only recursive
//! methods like tree traversals that only read self.field, this produces `&mut self`
//! when `&self` is correct. The cascade then contaminates callers.

fn get_wj_binary() -> String {
    env!("CARGO_BIN_EXE_wj").to_string()
}

fn compile_to_rust(wj_source: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    std::fs::write(&wj_path, wj_source).expect("write wj");
    std::fs::create_dir_all(&out_dir).expect("mkdir");

    let output = std::process::Command::new(get_wj_binary())
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let test_rs = out_dir.join("test.rs");
    Ok(std::fs::read_to_string(test_rs).expect("read"))
}

fn compile_project_to_rust(
    files: &[(&str, &str)],
) -> Result<std::collections::HashMap<String, String>, String> {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let src_dir = temp_dir.path().join("src");
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&src_dir).expect("mkdir src");
    std::fs::create_dir_all(&out_dir).expect("mkdir out");

    for (name, content) in files {
        let path = src_dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir parent");
        }
        std::fs::write(&path, content).expect("write");
    }

    let output = std::process::Command::new(get_wj_binary())
        .arg("build")
        .arg(&src_dir)
        .arg("--output")
        .arg(&out_dir)
        .arg("--library")
        .arg("--no-cargo")
        .output()
        .expect("wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let mut result = std::collections::HashMap::new();
    for (name, _) in files {
        let rs_name = name.replace(".wj", ".rs");
        let rs_path = out_dir.join(&rs_name);
        if rs_path.exists() {
            result.insert(rs_name, std::fs::read_to_string(rs_path).expect("read"));
        }
    }
    Ok(result)
}

#[test]
fn test_readonly_recursive_method_gets_ref_self() {
    let src = r#"
pub struct Tree {
    value: i32,
    left: i32,
    right: i32,
}

impl Tree {
    pub fn sum(self) -> i32 {
        if self.left < 0 {
            self.value
        } else {
            self.value + self.sum_child(self.left) + self.sum_child(self.right)
        }
    }

    fn sum_child(self, child_id: i32) -> i32 {
        if child_id < 0 {
            0
        } else {
            self.value
        }
    }
}
"#;

    let result = compile_to_rust(src);
    assert!(
        result.is_ok(),
        "Should compile: {:?}",
        result.as_ref().err()
    );
    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn sum(&self)"),
        "Read-only method should get &self, got:\n{}",
        rust
    );
}

#[test]
fn test_self_recursive_readonly_method_gets_ref_self() {
    let src = r#"
pub struct Evaluator {
    data: i32,
}

impl Evaluator {
    pub fn evaluate(self, depth: i32) -> i32 {
        self.evaluate_node(self.data, depth)
    }

    fn evaluate_node(self, node_id: i32, depth: i32) -> i32 {
        if depth <= 0 {
            return node_id
        }
        let left = self.evaluate_node(node_id + 1, depth - 1)
        let right = self.evaluate_node(node_id + 2, depth - 1)
        left + right
    }
}
"#;

    let result = compile_to_rust(src);
    assert!(
        result.is_ok(),
        "Should compile: {:?}",
        result.as_ref().err()
    );
    let rust = result.unwrap();
    assert!(
        rust.contains("fn evaluate_node(&self,"),
        "Read-only recursive method should get &self, not &mut self.\nGenerated:\n{}",
        rust
    );
    assert!(
        rust.contains("fn evaluate(&self,"),
        "Caller of read-only recursive method should also get &self.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_mutating_recursive_method_still_gets_mut_self() {
    let src = r#"
pub struct Counter {
    count: i32,
}

impl Counter {
    pub fn count_down(self, n: i32) {
        if n <= 0 {
            return
        }
        self.count = self.count + 1
        self.count_down(n - 1)
    }
}
"#;

    let result = compile_to_rust(src);
    assert!(
        result.is_ok(),
        "Should compile: {:?}",
        result.as_ref().err()
    );
    let rust = result.unwrap();
    assert!(
        rust.contains("fn count_down(&mut self,"),
        "Mutating recursive method must still get &mut self.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_cross_file_recursive_readonly_cascade() {
    let files = &[
        (
            "evaluator.wj",
            r#"
pub struct CsgEvaluator {
    root_id: i32,
}

impl CsgEvaluator {
    pub fn new(root: i32) -> CsgEvaluator {
        CsgEvaluator { root_id: root }
    }

    pub fn evaluate(self, x: f32) -> f32 {
        self.evaluate_node(self.root_id, x)
    }

    fn evaluate_node(self, node_id: i32, x: f32) -> f32 {
        if node_id < 0 {
            return 0.0
        }
        let left = self.evaluate_node(node_id - 1, x)
        let right = self.evaluate_node(node_id - 2, x)
        left + right + x
    }

    pub fn material_at(self, x: f32) -> i32 {
        let dist = self.evaluate(x)
        if dist < 0.0 { 1 } else { 0 }
    }
}
"#,
        ),
        (
            "voxelizer.wj",
            r#"
use crate::evaluator::CsgEvaluator

pub struct Voxelizer {
    evaluator: CsgEvaluator,
}

impl Voxelizer {
    pub fn new(root: i32) -> Voxelizer {
        Voxelizer { evaluator: CsgEvaluator::new(root) }
    }

    pub fn voxelize(self, size: i32) -> i32 {
        let mut total = 0
        for x in 0..size {
            let mat = self.evaluator.material_at(x as f32)
            total = total + mat
        }
        total
    }
}
"#,
        ),
    ];

    let result = compile_project_to_rust(files);
    assert!(
        result.is_ok(),
        "Should compile: {:?}",
        result.as_ref().err()
    );
    let outputs = result.unwrap();

    let evaluator_rs = outputs
        .get("evaluator.rs")
        .expect("evaluator.rs should exist");
    assert!(
        evaluator_rs.contains("fn evaluate_node(&self,"),
        "Read-only recursive evaluate_node should get &self.\nGenerated:\n{}",
        evaluator_rs
    );
    assert!(
        evaluator_rs.contains("fn material_at(&self,"),
        "material_at (calls read-only recursive) should get &self.\nGenerated:\n{}",
        evaluator_rs
    );

    let voxelizer_rs = outputs
        .get("voxelizer.rs")
        .expect("voxelizer.rs should exist");
    assert!(
        voxelizer_rs.contains("fn voxelize(&self,"),
        "voxelize (calls self.evaluator.material_at) should get &self.\nGenerated:\n{}",
        voxelizer_rs
    );
}
