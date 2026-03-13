/// TDD Test: Vec<String> indexing generates & (auto-borrow) for non-Copy types
///
/// Bug: `let line = lines[i]` where lines: Vec<String> generates move instead of borrow
/// Root cause: Rust doesn't allow moving out of Vec index (E0507)
/// Fix: Generate &vec[idx] (auto-borrow) for non-Copy - zero-cost, idiomatic
///      Generate vec[idx].clone() only when owned value needed (e.g. struct literal)
///
/// Discovered via dogfooding: breach-protocol save_manager.wj (split returns Vec<String>)

use std::process::Command;

fn compile_wj_to_rust_and_check(source: &str) -> (String, bool) {
    let dir = std::env::temp_dir().join(format!(
        "wj_vec_index_borrow_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
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

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let bin_output = dir.join("test_bin");
    let rustc = Command::new("rustc")
        .args(["--edition", "2021", "-o", bin_output.to_str().unwrap()])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

#[test]
fn test_vec_string_index_generates_borrow() {
    // Vec<String> indexing - need & (auto-borrow) because String is not Copy
    // Prefer & over .clone() - zero-cost, idiomatic
    let source = r#"
pub fn get_line(lines: Vec<string>, index: i32) -> string {
    let line = lines[index]
    return line
}

fn main() {
    let lines = vec!["a".to_string(), "b".to_string()]
    let x = get_line(lines, 0)
}
"#;

    let (rust, compiles) = compile_wj_to_rust_and_check(source);

    // Should generate &lines[index] (auto-borrow) - NOT raw lines[index] (E0507)
    assert!(
        rust.contains("&lines[") || rust.contains("& lines["),
        "Vec<String> indexing should generate & (auto-borrow), got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_vec_int_index_remains_direct() {
    // Vec<i32> indexing - no .clone() needed (Copy type)
    let source = r#"
pub fn get_int(numbers: Vec<i32>, index: i32) -> i32 {
    let num = numbers[index]
    return num
}

fn main() {
    let nums = vec![1, 2, 3]
    let x = get_int(nums, 0)
}
"#;

    let (rust, compiles) = compile_wj_to_rust_and_check(source);

    // i32 is Copy - get_int body should have numbers[index] without .clone()
    let get_int_body = rust
        .split("fn get_int")
        .nth(1)
        .unwrap_or("")
        .split("fn main")
        .next()
        .unwrap_or("");
    assert!(
        !get_int_body.contains("numbers[") || !get_int_body.contains("].clone()"),
        "Vec<i32> indexing should NOT add .clone() (Copy type), got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_local_var_vec_string_index_generates_borrow() {
    // Real-world case: local var from function returning Vec<String>
    // Simulates: let lines = split(...); let line = lines[i]
    let source = r#"
pub fn get_parts(text: string) -> Vec<string> {
    vec![text.to_string()]
}

pub fn parse_first(text: string) -> string {
    let parts = get_parts(text)
    let first = parts[0]
    return first
}

fn main() {}
"#;

    let (rust, compiles) = compile_wj_to_rust_and_check(source);

    // parts[0] where parts: Vec<String> needs & (auto-borrow) to avoid E0507
    assert!(
        rust.contains("&parts[") || rust.contains("& parts["),
        "Vec<String> from local var indexing should generate & (auto-borrow), got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}
