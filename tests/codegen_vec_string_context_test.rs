/// TDD Test: Vec<String> index and String/&str type mismatches
///
/// Bug fixes:
/// 1. Vec<String> index in struct literal needs .clone() (owned String field)
/// 2. String → &str auto-borrow for extern fn params
/// 3. String concatenation with Vec elements (result += parts[j])
use std::process::Command;

fn compile_wj_to_rust(source: &str, test_name: &str) -> String {
    let dir = std::env::temp_dir()
        .join(format!("wj_vec_string_ctx_{}_{}", test_name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    std::fs::read_to_string(&main_rs).unwrap_or_default()
}

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

    let rust = compile_wj_to_rust(source, "struct_clone");

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
extern fn hash(data: string) -> string

pub fn compute_hash(data: string) -> string {
    return hash(data)
}

fn main() {
    let h = compute_hash("test".to_string())
    println(h)
}
"#;

    let rust = compile_wj_to_rust(source, "extern_borrow");

    // Should auto-borrow String → &str for extern
    assert!(
        rust.contains("hash(&data)"),
        "Extern fn with String arg should get & for &str param\nGenerated:\n{}",
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

    let rust = compile_wj_to_rust(source, "string_concat");

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
