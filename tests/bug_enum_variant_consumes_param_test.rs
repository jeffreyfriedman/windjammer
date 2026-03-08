use std::fs;
/// TDD test: Parameters passed to enum variant constructors should stay Owned
///
/// Bug: `ObjectiveType::Kill(enemy_type, count)` - the `enemy_type` parameter
/// gets inferred as Borrowed (&String) instead of Owned (String), even though
/// the enum variant constructor consumes (moves) the parameter.
///
/// Root Cause: The `is_stored` check in the analyzer doesn't detect parameters
/// used as arguments to enum variant constructors in arbitrary expression positions
/// (e.g., inside other function call arguments, let bindings, etc.)
///
/// Fix: Add recursive scanning in `is_stored` to detect enum variant constructors
/// that consume the parameter.
use std::process::Command;

fn transpile_wj(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    // Use CARGO_BIN_EXE_wj for cross-platform compatibility (Windows CI fix)
    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    // Check compilation status
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Compilation failed:\nSTDERR:\n{}\nSTDOUT:\n{}",
            stderr, stdout
        );
    }

    let rust_file = out_dir.join("test.rs");
    let content = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    // Clean up temp directory
    let _ = fs::remove_dir_all(&test_dir);

    content
}

#[test]
fn test_enum_variant_constructor_consumes_param() {
    let source = r#"
pub enum Shape {
    Circle(f32),
    Named(String),
}

pub fn make_named(name: String) -> Shape {
    Shape::Named(name)
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // `name` should stay owned (String) since Shape::Named consumes it
    assert!(
        generated.contains("fn make_named(name: String)"),
        "Parameter consumed by enum variant should stay Owned. Got:\n{}",
        generated
    );
}

#[test]
fn test_enum_variant_in_nested_call() {
    let source = r#"
pub enum ObjectiveType {
    Kill(String, i32),
}

pub struct Objective {
    obj_type: ObjectiveType,
    count: i32,
}

impl Objective {
    pub fn new(obj_type: ObjectiveType, count: i32) -> Objective {
        Objective { obj_type, count }
    }
}

pub fn create_kill(enemy_type: String, count: i32) -> Objective {
    let desc = format!("Kill {} {}", count, enemy_type);
    Objective::new(ObjectiveType::Kill(enemy_type, count), count)
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // `enemy_type` should stay owned since it's consumed by Kill variant
    assert!(
        generated.contains("fn create_kill(enemy_type: String"),
        "Parameter consumed by enum variant in nested call should stay Owned. Got:\n{}",
        generated
    );
}

#[test]
fn test_enum_variant_multi_statement_with_format_reads() {
    // Match the exact quest.wj pattern: multi-statement function with
    // format!() reads before enum variant constructor storage
    let source = r#"
pub enum ObjectiveType {
    Kill(String, i32),
}

pub struct Quest {
    name: String,
    desc: String,
    quest_giver: String,
}

impl Quest {
    pub fn new(name: String, title: String, desc: String) -> Quest {
        Quest { name, desc, quest_giver: "".to_string() }
    }
    pub fn add_objective(self, obj: Objective) {}
}

pub struct Objective {
    name: String,
    desc: String,
    count: i32,
}

impl Objective {
    pub fn new_with_progress(name: String, desc: String, obj_type: &ObjectiveType, count: i32) -> Objective {
        Objective { name, desc, count }
    }
}

pub fn create_kill_quest(
    id: string,
    title: string,
    enemy_type: string,
    count: i32,
    quest_giver: string
) -> Quest {
    let mut quest = Quest::new(id.clone(), title, format!("Kill {} {}", count, enemy_type))
    quest.quest_giver = quest_giver

    let obj = Objective::new_with_progress(
        format!("{}_kill", id),
        format!("Kill {} {}", count, enemy_type),
        &ObjectiveType::Kill(enemy_type, count),
        count
    )
    quest.add_objective(obj)

    quest
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // enemy_type should stay owned because it's consumed by Kill variant
    assert!(
        !generated.contains("enemy_type: &String"),
        "enemy_type consumed by enum variant should NOT be &String. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("enemy_type: String"),
        "enemy_type consumed by enum variant should stay Owned (String). Got:\n{}",
        generated
    );
}

#[test]
fn test_enum_variant_with_ref_and_format_read() {
    // Exact pattern from quest.wj: parameter used in format!() AND enum variant with &
    let source = r#"
pub enum ObjectiveType {
    Kill(String, i32),
    Deliver(String, String),
}

pub struct Objective {
    name: String,
    desc: String,
    obj_type: ObjectiveType,
    count: i32,
}

impl Objective {
    pub fn new_with_progress(name: String, desc: String, obj_type: &ObjectiveType, count: i32) -> Objective {
        Objective { name, desc, obj_type: ObjectiveType::Kill("".to_string(), 0), count }
    }

    pub fn new(name: String, desc: String, obj_type: ObjectiveType) -> Objective {
        Objective { name, desc, obj_type, count: 0 }
    }
}

pub fn create_kill_quest(enemy_type: String, count: i32) -> Objective {
    Objective::new_with_progress(
        format!("kill_{}", enemy_type),
        format!("Kill {} {}", count, enemy_type),
        &ObjectiveType::Kill(enemy_type, count),
        count
    )
}

pub fn create_delivery_quest(item_id: String, recipient: String) -> Objective {
    Objective::new(
        format!("deliver_{}", item_id),
        format!("Deliver {} to {}", item_id, recipient),
        ObjectiveType::Deliver(item_id, recipient)
    )
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // enemy_type should stay owned - consumed by Kill variant
    assert!(
        generated.contains("fn create_kill_quest(enemy_type: String"),
        "enemy_type consumed by enum variant should stay Owned. Got:\n{}",
        generated
    );

    // item_id and recipient should stay owned - consumed by Deliver variant
    assert!(
        generated.contains("fn create_delivery_quest(item_id: String, recipient: String"),
        "item_id consumed by enum variant should stay Owned. Got:\n{}",
        generated
    );
}
