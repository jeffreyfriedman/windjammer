/// TDD Test: Option field access without moving
/// 
/// Problem: node.children.unwrap() moves from borrowed reference
/// 
/// Example:
/// ```windjammer
/// struct Node {
///     pub children: Option<Vec<Node>>
/// }
/// 
/// fn get_first(node: Node) -> Node {
///     let children = node.children.unwrap()  // ERROR: moves from &Node
///     children[0]
/// }
/// ```
/// 
/// Solution: Compiler should auto-insert .as_ref() for Option on borrowed fields
/// node.children.unwrap() -> node.children.as_ref().unwrap()

use std::fs;
use std::process::Command;

#[test]
fn test_option_field_unwrap_on_borrowed_param() {
    let source = r#"
struct Node {
    pub value: int,
    pub children: Option<Vec<int>>
}

fn get_sum(node: Node) -> int {
    if node.children.is_some() {
        let children = node.children.unwrap()
        children[0] + children[1]
    } else {
        0
    }
}

fn main() {
    let n = Node { value: 1, children: Some(vec![2, 3]) }
    let sum = get_sum(n)
    assert_eq(sum, 5)
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
    
    // Verify Option field access generates .as_ref() or .clone()
    assert!(
        generated.contains(".as_ref().unwrap()") || generated.contains(".clone().unwrap()"),
        "Option::unwrap on borrowed field should use .as_ref() or .clone()"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}
