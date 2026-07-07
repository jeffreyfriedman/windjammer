#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

// Bug: When a Copy struct is passed as an argument to a method that takes it
// owned, the codegen incorrectly generates `&val` instead of `val`.
//
// Example:
//   fn find_index(self, id: NodeId) -> i32 { ... }
//   fn remove(self, id: NodeId) {
//       let idx = self.find_index(id)   // WJ source: pass owned
//   }
//
// Generated (wrong):
//   fn remove(&mut self, id: NodeId) {
//       let idx = self.find_index(&id);  // BUG: adds & to Copy type
//   }
//
// Expected:
//   fn remove(&mut self, id: NodeId) {
//       let idx = self.find_index(id);   // CORRECT: Copy type, no &
//   }

#[test]
fn copy_struct_arg_not_borrowed_at_call_site() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        dir.path().join("wj.toml"),
        "[package]\nname = \"test_copy_arg\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    // NodeId in its own file (like game-core's behavior_tree/node_id.wj)
    std::fs::write(
        src.join("node_id.wj"),
        r#"
pub struct NodeId {
    pub value: i32
}

impl NodeId {
    pub fn value(self) -> i32 {
        self.value
    }
}
"#,
    )
    .unwrap();

    // Container uses NodeId from another file
    std::fs::write(
        src.join("container.wj"),
        r#"
use crate::node_id::NodeId

pub struct Container {
    ids: Vec<NodeId>
}

impl Container {
    pub fn new() -> Container {
        Container { ids: Vec::new() }
    }

    fn find_index(self, id: NodeId) -> i32 {
        let mut i = 0
        while i < self.ids.len() {
            if self.ids[i].value() == id.value() {
                return i as i32
            }
            i = i + 1
        }
        -1
    }

    pub fn has_item(self, id: NodeId) -> bool {
        self.find_index(id) >= 0
    }

    pub fn remove_item(self, id: NodeId) {
        let idx = self.find_index(id)
        if idx >= 0 {
            self.ids.remove(idx as usize)
        }
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("main.wj"),
        r#"
use crate::node_id::NodeId
use crate::container::Container

fn main() {
    let mut c = Container::new()
    let id = NodeId { value: 42 }
    c.ids.push(id)
    let found = c.has_item(id)
    println!("{}", found)
}
"#,
    )
    .unwrap();

    let wj = std::path::PathBuf::from(env!("CARGO_BIN_EXE_wj"));

    let output = std::process::Command::new(&wj)
        .arg("build")
        .arg("--no-cargo")
        .arg(&src)
        .current_dir(dir.path())
        .output()
        .expect("wj build failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "wj build failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    let build_dir = dir.path().join("build");
    let container_rs = find_generated_rs(&build_dir, "container");
    assert!(
        !container_rs.is_empty(),
        "Could not find generated container.rs in {:?}",
        build_dir
    );

    // The key assertion: when calling find_index(id) where id is a Copy struct
    // from another file, the generated Rust should NOT add & to the argument
    let has_borrow_call = container_rs.contains("find_index(&id)")
        || container_rs.contains("find_index(& id)")
        || container_rs.contains("find_index(&self.");

    assert!(
        !has_borrow_call,
        "Generated Rust adds & to Copy struct argument at cross-file call site!\n\
         This is wrong: Copy types should be passed by value, not by reference.\n\
         Generated container.rs:\n{}",
        container_rs
    );
}

fn find_generated_rs(build_dir: &std::path::Path, name: &str) -> String {
    let target = format!("{}.rs", name);
    fn search(dir: &std::path::Path, target: &str) -> Option<String> {
        if !dir.exists() {
            return None;
        }
        for entry in std::fs::read_dir(dir).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = search(&path, target) {
                    return Some(found);
                }
            } else if path.file_name().map(|n| n == target).unwrap_or(false) {
                return std::fs::read_to_string(&path).ok();
            }
        }
        None
    }
    search(build_dir, &target).unwrap_or_default()
}
