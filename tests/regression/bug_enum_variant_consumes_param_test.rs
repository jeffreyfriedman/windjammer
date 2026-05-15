#![cfg(not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
)))]

#[path = "../common/test_utils.rs"]
mod test_utils;

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

    let generated = test_utils::compile_single(source);
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

    let generated = test_utils::compile_single(source);
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

    let generated = test_utils::compile_single(source);
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

    let generated = test_utils::compile_single(source);
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
