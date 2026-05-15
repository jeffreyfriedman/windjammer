/// TDD Test: Parameter ownership for values read then moved into collections
///
/// Bug: When a function parameter is used in two steps:
///   1. Read via method call (e.g., param.id())
///   2. Moved into a collection (e.g., map.insert(key, param))
///
/// The compiler incorrectly infers &mut instead of owned, because the
/// method call triggers mutation detection. But .id() is a &self method,
/// and insert() needs the owned value.
///
/// Windjammer source:
/// ```wj
/// pub fn add_quest(self, quest: Quest) {
///     let id = quest.id()
///     self.quests.insert(id, quest)
/// }
/// ```
///
/// Expected Rust output:
/// ```rust
/// pub fn add_quest(&mut self, quest: Quest) {
///     let id = quest.id();
///     self.quests.insert(id, quest);
/// }
/// ```
///
/// Actual (broken) output:
/// ```rust
/// pub fn add_quest(&mut self, quest: &mut Quest) {
///     let id = quest.id();
///     self.quests.insert(id, quest);  // ERROR: expected Quest, found &mut Quest
/// }
/// ```
use std::path::PathBuf;
use std::process::Command;

fn compile_wj(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir().join(format!(
        "wj_owned_param_insert_test_{}_{}",
        std::process::id(),
        id
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let source_file = temp_dir.join("test_input.wj");
    std::fs::write(&source_file, source).unwrap();

    let output_dir = temp_dir.join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let wj_path = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = Command::new(&wj_path)
        .arg("build")
        .arg(source_file.to_str().unwrap())
        .arg("-o")
        .arg(output_dir.to_str().unwrap())
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_file = output_dir.join("test_input.rs");
    std::fs::read_to_string(&rs_file).expect("Failed to read generated Rust file")
}

#[test]
fn test_owned_param_read_then_insert() {
    let source = r#"
use std::collections::HashMap

struct Quest {
    id: string,
    title: string,
}

impl Quest {
    pub fn id(self) -> string {
        self.id
    }
}

struct QuestManager {
    quests: HashMap<string, Quest>,
}

impl QuestManager {
    pub fn add_quest(self, quest: Quest) {
        let id = quest.id()
        self.quests.insert(id, quest)
    }
}
"#;

    let rust = compile_wj(source);

    // The quest parameter must be owned (not &mut Quest)
    assert!(
        rust.contains("quest: Quest"),
        "Expected owned 'quest: Quest' parameter, but got:\n{}",
        rust
    );
    assert!(
        !rust.contains("quest: &mut Quest"),
        "Parameter should NOT be &mut Quest:\n{}",
        rust
    );
    assert!(
        !rust.contains("quest: &Quest"),
        "Parameter should NOT be &Quest:\n{}",
        rust
    );
}

#[test]
fn test_owned_param_used_after_move_gets_cloned() {
    // When a non-Copy value's fields are accessed multiple times,
    // the compiler should generate valid Rust -- either by keeping the
    // parameter owned or by borrowing and auto-cloning field accesses.
    let source = r#"
struct Item {
    id: string,
    name: string,
}

impl Item {
    pub fn id(self) -> string {
        self.id
    }
}

fn process_item(item: Item) {
    let id = item.id()
    let name = item.name
    println!("{}: {}", id, name)
}
"#;

    let rust = compile_wj(source);

    // Either ownership strategy is valid:
    //   1. item: Item  (owned, fields moved/consumed)
    //   2. item: &Item (borrowed, fields auto-cloned)
    let has_valid_sig = rust.contains("item: Item")
        || rust.contains("mut item: Item")
        || rust.contains("item: &Item");
    assert!(
        has_valid_sig,
        "Expected valid parameter signature (Item or &Item), but got:\n{}",
        rust
    );

    // If borrowed, field accesses must be cloned to compile
    if rust.contains("item: &Item") {
        assert!(
            rust.contains(".clone()"),
            "Borrowed parameter requires auto-cloned field access, but got:\n{}",
            rust
        );
    }
}

#[test]
fn test_self_field_reassignment_clones() {
    // self.field = self.other_field should auto-clone
    let source = r#"
struct Dialog {
    current_node_id: string,
    start_node_id: string,
}

impl Dialog {
    pub fn reset(self) {
        self.current_node_id = self.start_node_id
    }
}
"#;

    let rust = compile_wj(source);

    // Should generate a clone for the field reassignment
    assert!(
        rust.contains("self.start_node_id.clone()"),
        "Expected auto-clone for self.field = self.other_field, but got:\n{}",
        rust
    );
}
