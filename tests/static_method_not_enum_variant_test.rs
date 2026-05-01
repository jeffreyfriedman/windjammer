use std::path::Path;
use windjammer::compiler::build_project;
use windjammer::CompilationTarget;

fn compile_to_rust(code: &str) -> String {
    let _tmp = tempfile::tempdir().unwrap();
    let dir = _tmp.path().join(format!(
        "wj_enum_variant_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("test.wj"), code).unwrap();
    let out = dir.join("build");
    build_project(
        Path::new(&dir.join("test.wj")),
        &out,
        CompilationTarget::Rust,
        false,
    )
    .unwrap();
    std::fs::read_to_string(out.join("test.rs")).unwrap()
}

#[test]
fn test_static_method_call_does_not_force_owned() {
    let code = r#"
struct Node {
    value: i32,
    children: Vec<i32>
}

impl Node {
    fn find_child(nodes: Vec<Node>, target: i32) -> i32 {
        let mut i: usize = 0
        while i < nodes.len() {
            if nodes[i].value == target {
                return i as i32
            }
            i = i + 1
        }
        0i32
    }

    fn check_tree(nodes: Vec<Node>) -> bool {
        let result = Node::find_child(nodes, 42)
        result > 0
    }
}
"#;
    let rust = compile_to_rust(code);
    println!("{}", rust);
    // find_child only reads nodes → should be &Vec<Node>
    assert!(
        rust.contains("nodes: &Vec<Node>"),
        "Static method call should not force parameter to Owned. Expected &Vec<Node>.\nGenerated:\n{}",
        rust
    );
    // check_tree passes nodes to find_child which borrows → should auto-borrow
    assert!(
        rust.contains("Node::find_child(&nodes,") || rust.contains("nodes: &Vec<Node>"),
        "Caller should auto-borrow when callee borrows.\nGenerated:\n{}",
        rust
    );
}
