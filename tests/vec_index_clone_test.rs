#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD Test: Vec indexing of non-Copy types needs .clone()
//!
//! Bug: When passing `vec[i]` to a function that takes ownership of a non-Copy type,
//! the codegen generates `func(vec[i as usize])` which tries to move out of the Vec.
//! Rust doesn't allow moving out of an index.
//!
//! Expected: `func(vec[i as usize].clone())`
//! Actual:   `func(vec[i as usize])`

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_index_non_copy_passed_to_function() {
    let source = r#"
enum GameEvent {
    PlayerMove(f32, f32),
    ItemPickup(string),
    None,
}

fn describe_event(event: GameEvent) -> string {
    match event {
        GameEvent::PlayerMove(x, y) => "moved",
        GameEvent::ItemPickup(item) => "pickup",
        GameEvent::None => "none",
    }
}

fn main() {
    let events: Vec<GameEvent> = vec![
        GameEvent::PlayerMove(1.0, 2.0),
        GameEvent::ItemPickup("Sword"),
        GameEvent::None,
    ]
    let mut i = 0
    while i < 3 {
        let desc = describe_event(events[i])
        println("${desc}")
        i = i + 1
    }
}
"#;

    let (rust_code, compiles) = test_utils::compile_single_check(source);

    println!("Generated Rust:\n{}", rust_code);

    // The generated code should add .clone() when indexing into Vec<NonCopy>
    // and passing to a function that takes ownership
    assert!(
        rust_code.contains(".clone()"),
        "Expected .clone() for non-Copy type indexed from Vec.\nGenerated:\n{}",
        rust_code
    );

    assert!(compiles, "Generated Rust should compile successfully");
}

#[test]
fn test_vec_index_copy_type_no_clone() {
    let source = r#"
fn sum_vec(nums: Vec<i32>) -> i32 {
    let mut total = 0
    let mut i = 0
    while i < 3 {
        total = total + nums[i]
        i = i + 1
    }
    total
}

fn main() {
    let nums: Vec<i32> = vec![10, 20, 30]
    println("Sum: ${sum_vec(nums)}")
}
"#;

    let (rust_code, compiles) = test_utils::compile_single_check(source);

    println!("Generated Rust:\n{}", rust_code);

    // Copy types (i32) should NOT get .clone() when indexed
    // (nums[i] is fine because i32 is Copy)
    assert!(
        compiles,
        "Copy type Vec indexing should compile without .clone()"
    );
}
