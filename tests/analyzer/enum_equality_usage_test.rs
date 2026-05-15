/// TDD Test: Enum equality operator (==) usage in Windjammer code
///
/// Verifies that `a == b` and `a != b` work on unit enums and produce
/// valid Rust that compiles with rustc. The compiler derives PartialEq
/// for unit enums, so == should just work.
///
/// This test validates the entire pipeline: WJ source → generated Rust → rustc.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_unit_enum_equality_operator() {
    let source = r#"
enum Direction {
    North,
    South,
    East,
    West,
}

pub fn is_north(d: Direction) -> bool {
    d == Direction::North
}

pub fn is_not_south(d: Direction) -> bool {
    d != Direction::South
}
"#;
    let (info, ok) = test_utils::compile_single_check(source);
    assert!(ok, "Unit enum == should compile through rustc:\n{}", info);
}

#[test]
fn test_enum_equality_in_if_condition() {
    let source = r#"
enum Color {
    Red,
    Green,
    Blue,
}

pub fn color_name(c: Color) -> string {
    if c == Color::Red {
        "red"
    } else if c == Color::Green {
        "green"
    } else {
        "blue"
    }
}
"#;
    let (info, ok) = test_utils::compile_single_check(source);
    assert!(ok, "Enum == in if conditions should compile:\n{}", info);
}

#[test]
fn test_enum_equality_with_variable() {
    let source = r#"
enum State {
    Active,
    Paused,
    Stopped,
}

pub fn states_equal(a: State, b: State) -> bool {
    a == b
}

pub fn states_not_equal(a: State, b: State) -> bool {
    a != b
}
"#;
    let (info, ok) = test_utils::compile_single_check(source);
    assert!(ok, "Enum == between variables should compile:\n{}", info);
}
