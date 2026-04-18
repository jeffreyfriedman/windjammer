/// TDD Tests: String Type Unification (W0010)
///
/// Windjammer has ONE string type: `string`. The compiler should:
/// 1. Normalize `str`, `String`, `&str` to `Type::String` at parse time
/// 2. Emit W0010 warning for non-canonical string types
/// 3. Exempt extern fn declarations from the warning
/// 4. Generate identical codegen regardless of which spelling was used

use std::process::Command;

/// Compile .wj source and return (generated_rust, stderr_output)
fn compile_wj(source: &str, test_name: &str) -> (String, String) {
    let dir = std::env::temp_dir().join(format!(
        "wj_string_unify_{}_{}", test_name, std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
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

    let rust_code = std::fs::read_to_string(&main_rs).unwrap_or_default();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let _ = std::fs::remove_dir_all(&dir);
    (rust_code, stderr)
}

// --- W0010 Warning Tests ---

#[test]
fn test_w0010_fires_for_str_in_param() {
    let source = r#"
pub fn greet(name: str) {
    println!("{}", name)
}

pub fn main() {
    greet("world")
}
"#;
    let (_rust, stderr) = compile_wj(source, "w0010_str_param");
    assert!(
        stderr.contains("W0010"),
        "W0010 should fire for `str` in parameter type\nStderr:\n{}",
        stderr
    );
    assert!(
        stderr.contains("string"),
        "W0010 should suggest using `string`\nStderr:\n{}",
        stderr
    );
}

#[test]
fn test_w0010_fires_for_string_uppercase_in_return() {
    let source = r#"
pub fn make_name() -> String {
    "hello"
}

pub fn main() {
    let x = make_name()
}
"#;
    let (_rust, stderr) = compile_wj(source, "w0010_String_return");
    assert!(
        stderr.contains("W0010"),
        "W0010 should fire for `String` in return type\nStderr:\n{}",
        stderr
    );
}

#[test]
fn test_w0010_fires_for_ampersand_str_in_param() {
    let source = r#"
pub fn greet(name: &str) {
    println!("{}", name)
}

pub fn main() {
    greet("world")
}
"#;
    let (_rust, stderr) = compile_wj(source, "w0010_ref_str_param");
    // Should fire either W0010 or W0001 (explicit reference) -- at minimum one warning
    assert!(
        stderr.contains("W0010") || stderr.contains("W0001"),
        "Should warn about `&str` in parameter type\nStderr:\n{}",
        stderr
    );
}

#[test]
fn test_w0010_does_not_fire_for_canonical_string() {
    let source = r#"
pub fn greet(name: string) {
    println!("{}", name)
}

pub fn main() {
    greet("world")
}
"#;
    let (_rust, stderr) = compile_wj(source, "w0010_no_fire_string");
    assert!(
        !stderr.contains("W0010"),
        "W0010 should NOT fire for canonical `string` type\nStderr:\n{}",
        stderr
    );
}

#[test]
fn test_w0010_does_not_fire_for_extern_fn() {
    let source = r#"
extern fn get_name() -> str {
}

pub fn main() {
    let x = 42
}
"#;
    let (_rust, stderr) = compile_wj(source, "w0010_extern_exempt");
    assert!(
        !stderr.contains("W0010"),
        "W0010 should NOT fire for extern fn declarations\nStderr:\n{}",
        stderr
    );
}

// --- Parser Normalization Tests ---

#[test]
fn test_str_normalizes_to_same_codegen_as_string() {
    let source_str = r#"
pub struct Bag {
    items: Vec<str>,
}

impl Bag {
    pub fn new() -> Bag {
        Bag { items: Vec::new() }
    }
    pub fn add(self, item: str) {
        self.items.push(item)
    }
}

pub fn main() {
    let mut bag = Bag::new()
    bag.add("apple")
}
"#;
    let source_string = r#"
pub struct Bag {
    items: Vec<string>,
}

impl Bag {
    pub fn new() -> Bag {
        Bag { items: Vec::new() }
    }
    pub fn add(self, item: string) {
        self.items.push(item)
    }
}

pub fn main() {
    let mut bag = Bag::new()
    bag.add("apple")
}
"#;
    let (rust_str, _) = compile_wj(source_str, "normalize_str");
    let (rust_string, _) = compile_wj(source_string, "normalize_string");

    assert!(
        !rust_str.is_empty(),
        "Should generate code for `str` version"
    );
    assert!(
        !rust_string.is_empty(),
        "Should generate code for `string` version"
    );

    // Both should generate Vec<String> in Rust
    assert!(
        rust_str.contains("Vec<String>"),
        "Vec<str> should become Vec<String> in Rust\nGenerated:\n{}",
        rust_str
    );
    assert!(
        rust_string.contains("Vec<String>"),
        "Vec<string> should become Vec<String> in Rust\nGenerated:\n{}",
        rust_string
    );
}

#[test]
fn test_struct_field_str_becomes_string_in_rust() {
    let source = r#"
pub struct Person {
    name: str,
    title: str,
}

impl Person {
    pub fn new(name: str, title: str) -> Person {
        Person { name: name, title: title }
    }
}

pub fn main() {
    let p = Person::new("Alice", "Engineer")
}
"#;
    let (rust, _) = compile_wj(source, "struct_field_str");
    assert!(
        !rust.is_empty(),
        "Should generate code"
    );
    // Struct fields should be String (owned)
    assert!(
        rust.contains("name: String") || rust.contains("name : String"),
        "Struct field `name: str` should become `name: String` in Rust\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_vec_str_becomes_vec_string_in_rust() {
    let source = r#"
pub fn collect_names() -> Vec<str> {
    let mut names = Vec::new()
    names.push("Alice")
    names.push("Bob")
    names
}

pub fn main() {
    let names = collect_names()
}
"#;
    let (rust, _) = compile_wj(source, "vec_str_codegen");
    assert!(
        !rust.is_empty(),
        "Should generate code"
    );
    assert!(
        rust.contains("Vec<String>"),
        "Vec<str> should become Vec<String> in Rust\nGenerated:\n{}",
        rust
    );
}

// --- Regression Tests ---

#[test]
fn test_hashmap_string_key_insert_works() {
    let source = r#"
use std::collections::HashMap

pub struct Registry {
    items: HashMap<string, i32>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry { items: HashMap::new() }
    }
    pub fn register(self, name: string, value: i32) {
        self.items.insert(name, value)
    }
}

pub fn main() {
    let mut reg = Registry::new()
    reg.register("test", 42)
}
"#;
    let (rust, _) = compile_wj(source, "hashmap_string_key");
    assert!(
        !rust.is_empty(),
        "Should generate code for HashMap<string, i32>"
    );
    assert!(
        rust.contains("HashMap<String"),
        "HashMap key should be String\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_trait_impl_string_return_compiles() {
    let source = r#"
pub trait Named {
    fn name(self) -> string
}

pub struct Dog {
    breed: string,
}

impl Named for Dog {
    fn name(self) -> string {
        self.breed.clone()
    }
}

pub fn main() {
    let d = Dog { breed: "Labrador" }
}
"#;
    let (rust, _) = compile_wj(source, "trait_string_return");
    assert!(
        !rust.is_empty(),
        "Should generate code for trait with string return"
    );
}

// --- W0010 fires for struct fields and return types ---

#[test]
fn test_w0010_fires_for_str_in_struct_field() {
    let source = r#"
pub struct Config {
    name: str,
}

pub fn main() {
    let c = Config { name: "test" }
}
"#;
    let (_rust, stderr) = compile_wj(source, "w0010_struct_field");
    assert!(
        stderr.contains("W0010"),
        "W0010 should fire for `str` in struct field\nStderr:\n{}",
        stderr
    );
}

#[test]
fn test_w0010_fires_for_str_in_return_type() {
    let source = r#"
pub fn get_name() -> str {
    "hello"
}

pub fn main() {
    let x = get_name()
}
"#;
    let (_rust, stderr) = compile_wj(source, "w0010_return_str");
    assert!(
        stderr.contains("W0010"),
        "W0010 should fire for `str` in return type\nStderr:\n{}",
        stderr
    );
}
