#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_mutated_and_returned_vec_should_be_owned() {
    // A function that mutates a Vec and then returns it must take ownership.
    // Taking &mut would make the return type &mut Vec, which is a type mismatch.
    let output = test_utils::compile_single(
        r#"
fn sort_and_return(items: Vec<i32>) -> Vec<i32> {
    items.sort()
    items
}
"#,
    );
    assert!(
        output.contains("mut items: Vec<i32>") || output.contains("items: Vec<i32>"),
        "Parameter should be owned (not &mut Vec<i32>) when both mutated and returned. Got:\n{}",
        output
    );
    assert!(
        !output.contains("items: &mut Vec<i32>"),
        "Parameter should NOT be &mut when returned. Got:\n{}",
        output
    );
}

#[test]
fn test_mutated_only_vec_should_be_mut_borrowed() {
    // A function that mutates but does NOT return should use &mut (preserving existing behavior)
    let output = test_utils::compile_single(
        r#"
fn add_item(items: Vec<i32>, value: i32) {
    items.push(value)
}
"#,
    );
    assert!(
        output.contains("items: &mut Vec<i32>"),
        "Parameter should be &mut when mutated but not returned. Got:\n{}",
        output
    );
}

#[test]
fn test_mutated_and_returned_string_should_be_owned() {
    let output = test_utils::compile_single(
        r#"
fn append_and_return(s: String, suffix: String) -> String {
    s.push_str(suffix)
    s
}
"#,
    );
    assert!(
        !output.contains("s: &mut String"),
        "String param should be owned when both mutated and returned. Got:\n{}",
        output
    );
}

#[test]
fn test_mutated_returned_via_last_expression() {
    let output = test_utils::compile_single(
        r#"
fn process(data: Vec<i32>) -> Vec<i32> {
    data.push(42)
    data
}
"#,
    );
    assert!(
        !output.contains("data: &mut Vec<i32>"),
        "Parameter should be owned when mutated and returned as last expression. Got:\n{}",
        output
    );
}

#[test]
fn test_mutated_and_returned_custom_struct_vec() {
    // This is the actual failing pattern from game code:
    // a function that takes Vec<CustomStruct>, mutates it, and returns it
    let output = test_utils::compile_single(
        r#"
struct DrawCommand {
    material_id: i32,
    mesh_id: i32,
}

fn sort_draw_commands(commands: Vec<DrawCommand>) -> Vec<DrawCommand> {
    commands.sort()
    commands
}
"#,
    );
    assert!(
        !output.contains("commands: &mut Vec<DrawCommand>"),
        "Custom struct Vec param should be owned when mutated and returned. Got:\n{}",
        output
    );
}
