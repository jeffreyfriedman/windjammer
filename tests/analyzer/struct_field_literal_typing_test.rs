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

//! TDD: Integer literals in struct field initializers must match field types (u32, usize, u8).
//!
//! Root cause fixed: struct definitions inside `mod { }` were never registered in
//! `IntInference::struct_field_types`, so literals defaulted to i32 in generated Rust.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_u32_fields_top_level_struct() {
    let source = r#"
pub struct DialogueChoice {
    pub id: u32,
    pub next_id: u32
}

pub fn sample() -> DialogueChoice {
    DialogueChoice { id: 1, next_id: 2 }
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("id: 1_u32") && rust.contains("next_id: 2_u32"),
        "Expected u32 suffixes for u32 fields. Got:\n{}",
        rust
    );
    assert!(
        !rust.contains("id: 1_i32") && !rust.contains("next_id: 2_i32"),
        "Must not emit i32 for u32 fields. Got:\n{}",
        rust
    );
}

#[test]
fn test_u32_fields_struct_inside_nested_mod() {
    let source = r#"
pub mod dialogue {
    pub struct DialogueChoice {
        pub id: u32,
        pub next_id: u32
    }

    pub fn sample() -> DialogueChoice {
        DialogueChoice { id: 1, next_id: 2 }
    }
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("id: 1_u32") && rust.contains("next_id: 2_u32"),
        "Nested mod: struct field literals should use u32. Got:\n{}",
        rust
    );
    assert!(
        !rust.contains("id: 1_i32"),
        "Nested mod: must not default to i32. Got:\n{}",
        rust
    );
}

#[test]
fn test_usize_field_initializer() {
    let source = r#"
pub struct Buf {
    pub len: usize,
    pub cap: usize
}

pub fn empty() -> Buf {
    Buf { len: 0, cap: 16 }
}
"#;
    let rust = test_utils::compile_single(source);
    // usize field literals may be unsuffixed: Rust infers from field type (avoids E0308 elsewhere).
    assert!(
        (rust.contains("len: 0_usize") || rust.contains("len: 0,"))
            && (rust.contains("cap: 16_usize") || rust.contains("cap: 16")),
        "Expected usize field initializers (0 / 16, suffixed or inferred). Got:\n{}",
        rust
    );
    assert!(
        !rust.contains("len: 0_i32") && !rust.contains("cap: 16_i32"),
        "Must not use i32 literals for usize fields. Got:\n{}",
        rust
    );
}

#[test]
fn test_vec_of_structs_u32_literals() {
    let source = r#"
pub struct Choice {
    pub id: u32,
    pub text: String
}

pub fn choices() -> Vec<Choice> {
    vec![
        Choice { id: 1, text: "First".to_string() },
        Choice { id: 2, text: "Second".to_string() }
    ]
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("id: 1_u32") && rust.contains("id: 2_u32"),
        "vec! elements should keep u32 field literals. Got:\n{}",
        rust
    );
    assert!(!rust.contains("id: 1_i32"), "Got:\n{}", rust);
}

#[test]
fn test_nested_struct_u8_in_mod() {
    let source = r#"
pub mod color {
    pub struct Rgb {
        pub r: u8,
        pub g: u8,
        pub b: u8
    }

    pub fn red() -> Rgb {
        Rgb { r: 255, g: 0, b: 0 }
    }
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("r: 255_u8") && rust.contains("g: 0_u8") && rust.contains("b: 0_u8"),
        "u8 fields in mod should use _u8 literals. Got:\n{}",
        rust
    );
    assert!(!rust.contains("255_i32"), "Got:\n{}", rust);
}
