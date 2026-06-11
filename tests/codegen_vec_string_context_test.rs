#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// TDD Test: Vec<String> index and String/&str type mismatches
///
/// Bug fixes:
/// 1. Vec<String> index in struct literal needs .clone() (owned String field)
/// 2. String → &str auto-borrow for extern fn params
/// 3. String concatenation with Vec elements (result += parts[j])
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_string_index_in_struct_needs_clone() {
    // Struct field expects String, Vec index returns &String → need .clone()
    let source = r#"
pub struct Info {
    pub name: string,
}

pub fn make_info(names: Vec<string>, i: i32) -> Info {
    return Info { name: names[i] }
}

fn main() {
    let names = vec!["a".to_string(), "b".to_string()]
    let info = make_info(names, 0)
    println(info.name)
}
"#;

    let rust = test_utils::compile_single(source);

    // Struct field expects String, so need .clone()
    assert!(
        rust.contains("names[i as usize].clone()") || rust.contains("names[(i as usize)].clone()"),
        "Vec<String> index in struct should use .clone()\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_string_to_str_auto_borrow_extern_fn() {
    // extern fn hash(data: &str) called with String should auto-borrow
    let source = r#"
extern fn hash(data: String) -> String

pub fn compute_hash(data: string) -> string {
    return hash(data)
}

fn main() {
    let h = compute_hash("test".to_string())
    println(h)
}
"#;

    let rust = test_utils::compile_single(source);

    // Extern fn string args go through FFI conversion (string_to_ffi)
    assert!(
        rust.contains("hash(") && (rust.contains("string_to_ffi") || rust.contains("hash(&data)")),
        "Extern fn with String arg should use FFI conversion or auto-borrow\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_string_concat_with_vec_element() {
    // result += parts[j] should work: String += &str
    let source = r#"
pub fn join(parts: Vec<string>) -> string {
    let mut result = "".to_string()
    let mut j = 0
    while j < parts.len() {
        if j > 0 {
            result = result + "|"
        }
        result = result + parts[j]
        j = j + 1
    }
    return result
}

fn main() {
    let p = vec!["a".to_string(), "b".to_string()]
    println(join(p))
}
"#;

    let rust = test_utils::compile_single(source);

    // Should handle String + &str correctly: need & for Rust's String + &str
    // Accept: result + &parts[j], result + &parts[j].clone(), or result += &parts[j]
    let has_valid_concat = rust.contains("result + &parts")
        || rust.contains("result += &parts")
        || rust.contains("+ &parts[");
    assert!(
        has_valid_concat,
        "String concat with Vec element should use & for &str\nGenerated:\n{}",
        rust
    );
}
