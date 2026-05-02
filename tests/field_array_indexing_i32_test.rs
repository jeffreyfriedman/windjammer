// TDD Test: Field array indexing with i32 should auto-cast to usize

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_field_array_indexing_with_i32() {
    let source = r#"
struct Agent {
    neighbors: Vec<u64>
}

fn test_indexing() {
    let agent = Agent { neighbors: vec![1, 2, 3] }
    let i = 0
    let id = agent.neighbors[i]
    println!("ID: {}", id)
}
"#;

    let rust = test_utils::compile_single(source);
    println!("Generated Rust:\n{}", rust);

    assert!(
        rust.contains("agent.neighbors[i as usize]")
            || rust.contains("agent.neighbors[(i as usize)]")
            || rust.contains("agent.neighbors[i]"),
        "Should index into field array.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_vec_indexing_with_loop() {
    let source = r#"
struct SteeringAgent {
    neighbors: Vec<u64>
}

fn process_neighbors(agent: SteeringAgent) {
    let mut i = 0
    while i < agent.neighbors.len() {
        let neighbor_id = agent.neighbors[i]
        println!("{}", neighbor_id)
        i = i + 1
    }
}
"#;

    let rust = test_utils::compile_single(source);
    println!("Generated Rust:\n{}", rust);

    // Both `i as usize` and plain `i` are valid when i is inferred as usize
    assert!(
        rust.contains("agent.neighbors[i as usize]") || rust.contains("agent.neighbors[i]"),
        "Should index into vec in loop.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_struct_field_compound_assignment() {
    let source = r#"
struct Counter {
    value: usize
}

impl Counter {
    fn increment(self) {
        self.value = self.value + 1
    }
}
"#;

    let rust = test_utils::compile_single(source);
    println!("Generated Rust:\n{}", rust);

    assert!(
        rust.contains("self.value += 1_usize") || rust.contains("self.value += 1"),
        "Should generate compound assignment for usize field.\nGenerated:\n{}",
        rust
    );
}
