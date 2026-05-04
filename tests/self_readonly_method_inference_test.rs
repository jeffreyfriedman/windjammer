/// TDD: Methods calling readonly operations on self.field must infer &self, not &mut self.
///
/// The compiler must use the signature registry (not a hardcoded method name list)
/// to determine whether a method call on self.field is mutating. User-defined methods
/// with &self receivers should not trigger &mut self promotion.
///
/// This test ensures that:
/// 1. User-defined readonly methods on self.field produce &self
/// 2. User-defined mutating methods on self.field produce &mut self
/// 3. No hardcoded method name whitelist is needed
use std::process::Command;

fn compile_wj_to_rs(source: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let input = dir.path().join("test.wj");
    std::fs::write(&input, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target=rust")
        .arg("--no-cargo")
        .arg(&input)
        .arg("--output")
        .arg(dir.path().join("out"))
        .output()
        .expect("wj build failed");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let rs_path = dir.path().join("out").join("test.rs");
    let generated = std::fs::read_to_string(&rs_path).unwrap_or_default();

    // Keep temp dir alive
    std::mem::forget(dir);
    (success, generated, stderr)
}

#[test]
fn test_user_defined_readonly_method_infers_ref_self() {
    // `status()` only reads self.health (Copy) — should be &self
    // `name()` only reads self.name (non-Copy) — should be &self (with auto-clone)
    // Neither should trigger &mut self
    let source = r#"
struct Player {
    name: String,
    health: i32,
}

impl Player {
    fn new(name: String, health: i32) -> Player {
        Player { name, health }
    }

    fn status(self) -> i32 {
        self.health
    }

    fn name(self) -> String {
        self.name
    }

    fn is_alive(self) -> bool {
        self.health > 0
    }
}

fn main() {
    let player = Player::new("hero".to_string(), 100)
    println!("{}", player.status())
    println!("{}", player.is_alive())
}
"#;

    let (success, generated, stderr) = compile_wj_to_rs(source);
    assert!(success, "Compilation failed: {}", stderr);

    // status() should be &self, NOT &mut self
    assert!(
        generated.contains("fn status(&self)"),
        "status() should use &self since it only reads self.health.\nGenerated:\n{}",
        generated
    );

    // is_alive() should be &self, NOT &mut self
    assert!(
        generated.contains("fn is_alive(&self)"),
        "is_alive() should use &self since it only reads self.health.\nGenerated:\n{}",
        generated
    );

    // name() should be &self (with auto-clone), NOT &mut self or owned self
    assert!(
        generated.contains("fn name(&self)"),
        "name() should use &self (codegen auto-clones non-Copy return).\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_user_defined_readonly_method_with_option_field() {
    // This is the octree case: is_leaf() calls self.children.is_none()
    // is_none() is &self on Option — should NOT promote to &mut self
    let source = r#"
struct TreeNode {
    value: i32,
    children: Option<Vec<TreeNode>>,
}

impl TreeNode {
    fn new() -> TreeNode {
        TreeNode { value: 0, children: None }
    }

    fn is_leaf(self) -> bool {
        self.children.is_none()
    }

    fn value(self) -> i32 {
        self.value
    }
}

fn main() {
    let node = TreeNode::new()
    assert!(node.is_leaf())
}
"#;

    let (success, generated, stderr) = compile_wj_to_rs(source);
    assert!(success, "Compilation failed: {}", stderr);

    assert!(
        generated.contains("fn is_leaf(&self)"),
        "is_leaf() should use &self, not &mut self.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_mutating_method_still_gets_mut_self() {
    // subdivide() assigns to self.children — must be &mut self
    let source = r#"
struct TreeNode {
    value: i32,
    children: Option<Vec<TreeNode>>,
}

impl TreeNode {
    fn new() -> TreeNode {
        TreeNode { value: 0, children: None }
    }

    fn subdivide(self) {
        let mut kids = Vec::new()
        kids.push(TreeNode::new())
        self.children = Some(kids)
    }
}

fn main() {
    let mut node = TreeNode::new()
    node.subdivide()
}
"#;

    let (success, generated, stderr) = compile_wj_to_rs(source);
    assert!(success, "Compilation failed: {}", stderr);

    assert!(
        generated.contains("fn subdivide(&mut self)"),
        "subdivide() should use &mut self since it assigns to self.children.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_method_calling_custom_readonly_method_on_field() {
    // validate() calls self.config.is_valid() — a user-defined &self method.
    // This must NOT require a hardcoded whitelist entry for "is_valid".
    let source = r#"
struct Config {
    max_size: i32,
}

impl Config {
    fn new() -> Config {
        Config { max_size: 100 }
    }

    fn is_valid(self) -> bool {
        self.max_size > 0
    }
}

struct App {
    config: Config,
    name: String,
}

impl App {
    fn new() -> App {
        App { config: Config::new(), name: "test".to_string() }
    }

    fn validate(self) -> bool {
        self.config.is_valid()
    }
}

fn main() {
    let app = App::new()
    println!("{}", app.validate())
}
"#;

    let (success, generated, stderr) = compile_wj_to_rs(source);
    assert!(success, "Compilation failed: {}", stderr);

    // validate() should be &self — self.config.is_valid() is a readonly call
    assert!(
        generated.contains("fn validate(&self)"),
        "validate() should use &self since is_valid() is readonly.\nGenerated:\n{}",
        generated
    );
}
