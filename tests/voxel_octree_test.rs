//! TDD Phase 3: SVO Octree - Sparse Voxel Octree
//!
//! Goal: 10x+ memory compression for sparse voxel grids
//! Tests: OctreeNode, grid-to-octree conversion, memory efficiency

use std::path::PathBuf;
use std::fs;

fn compile_and_run(source: &str, _test_name: &str) -> (bool, String) {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_voxel_octree_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    fs::write(test_dir.join("octree.wj"), source).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("octree.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        fs::remove_dir_all(test_dir).ok();
        return (false, format!("STDERR:\n{}\nSTDOUT:\n{}", stderr, stdout));
    }

    // Run the compiled binary
    let binary_path = test_dir.join("build/octree");
    let run_output = std::process::Command::new(&binary_path)
        .current_dir(&test_dir)
        .output();

    fs::remove_dir_all(test_dir).ok();

    match run_output {
        Ok(out) => {
            let run_stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let run_stderr = String::from_utf8_lossy(&out.stderr).to_string();
            (
                out.status.success(),
                format!("STDOUT:\n{}\nSTDERR:\n{}", run_stdout, run_stderr),
            )
        }
        Err(e) => (false, format!("Failed to run: {}", e)),
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_octree_node_creation() {
    let code = r#"
struct VoxelGrid {
    width: i32,
    height: i32,
    depth: i32,
    data: Vec<u8>,
}

impl VoxelGrid {
    fn new(width: i32, height: i32, depth: i32) -> VoxelGrid {
        let size = (width * height * depth) as usize;
        let mut data = Vec::new();
        for i in 0..size {
            data.push(0);
        }
        VoxelGrid { width, height, depth, data }
    }
    fn get(self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || x >= self.width || y < 0 || y >= self.height || z < 0 || z >= self.depth {
            return 0;
        }
        let idx = (x + y * self.width + z * self.width * self.height) as usize;
        self.data[idx]
    }
    fn set(self, x: i32, y: i32, z: i32, value: u8) {
        if x < 0 || x >= self.width || y < 0 || y >= self.height || z < 0 || z >= self.depth {
            return;
        }
        let idx = (x + y * self.width + z * self.width * self.height) as usize;
        self.data[idx] = value;
    }
    fn width(self) -> i32 { self.width }
}

struct OctreeNode {
    value: u8,
    children: Option<Vec<OctreeNode>>,
}

impl OctreeNode {
    fn new() -> OctreeNode {
        OctreeNode { value: 0, children: None }
    }
    fn is_leaf(self) -> bool {
        self.children.is_none()
    }
    fn value(self) -> u8 { self.value }
    fn child_count(self) -> i32 {
        self.children.as_ref().map(|c| c.len() as i32).unwrap_or(0)
    }
    fn subdivide(self) {
        if !self.is_leaf() { return; }
        let mut children = Vec::new();
        for _ in 0..8 {
            children.push(OctreeNode::new());
        }
        self.children = Some(children);
    }
}

fn main() {
    let node = OctreeNode::new();
    assert!(node.is_leaf());
    assert_eq!(node.value(), 0);
    println("test_octree_node_creation: PASS");
}
"#;

    let (success, output) = compile_and_run(code, "octree_node");
    assert!(success, "Compilation/run failed:\n{}", output);
    assert!(output.contains("PASS"), "Test should pass:\n{}", output);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_octree_subdivision() {
    let code = r#"
struct OctreeNode {
    value: u8,
    children: Option<Vec<OctreeNode>>,
}

impl OctreeNode {
    fn new() -> OctreeNode {
        OctreeNode { value: 0, children: None }
    }
    fn is_leaf(self) -> bool {
        self.children.is_none()
    }
    fn child_count(self) -> i32 {
        self.children.as_ref().map(|c| c.len() as i32).unwrap_or(0)
    }
    fn subdivide(self) {
        if !self.is_leaf() { return; }
        let mut children = Vec::new();
        for _ in 0..8 {
            children.push(OctreeNode::new());
        }
        self.children = Some(children);
    }
}

fn main() {
    let mut node = OctreeNode::new();
    node.subdivide();
    assert!(!node.is_leaf());
    assert_eq!(node.child_count(), 8);
    println("test_octree_subdivision: PASS");
}
"#;

    let (success, output) = compile_and_run(code, "octree_subdivision");
    assert!(success, "Compilation/run failed:\n{}", output);
    assert!(output.contains("PASS"), "Test should pass:\n{}", output);
}
