/// TDD Test: str params stored in owned containers need .to_string()
///
/// Bug: When a Windjammer function takes `str` (codegen: `&str`) and stores it
/// in Vec<String>, HashMap<String,...>, Option<String>, or a String struct field,
/// the generated Rust must add `.to_string()` conversion.
///
/// Root cause: The codegen was missing `.to_string()` for variable arguments
/// (it handled literals but not variable identifiers typed as `&str`).
///
/// Affected patterns:
///   - HashMap<String, V>::insert(key, val) where key is `&str`
///   - Vec<String>::push(val) where val is `&str`
///   - self.field = Some(val) where field is Option<String> and val is `&str`
///   - self.field = val where field is String and val is `&str`
///   - Struct { field: val } where field is String and val is `&str`
///
/// Fix: Use `string` (owned) type in .wj source when the value will be stored.
/// Future compiler improvement: auto-insert .to_string() when `str` is stored.
use std::process::Command;

fn compile_wj_to_rust(source: &str, test_name: &str) -> String {
    let _tmp = tempfile::tempdir().unwrap();
    let dir = _tmp.path().join(format!(
        "wj_str_owned_container_{}_{}",
        test_name,
        std::process::id()
    ));

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
fn test_string_param_stored_in_vec_push() {
    let source = r#"
pub struct Registry {
    items: Vec<string>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry { items: Vec::new() }
    }

    pub fn add(self, name: string) {
        self.items.push(name)
    }
}

fn main() {
    let mut r = Registry::new()
    r.add("hello")
}
"#;

    let rust = compile_wj_to_rust(source, "vec_push_owned");

    assert!(
        !rust.is_empty(),
        "Compiler should generate code for Vec<String> push with string param"
    );

    assert!(
        !rust.contains("name: str)"),
        "Must not emit bare 'str' (unsized) in param\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_string_param_stored_in_hashmap_insert() {
    let source = r#"
pub struct DataStore {
    data: HashMap<string, i32>,
}

impl DataStore {
    pub fn new() -> DataStore {
        DataStore { data: HashMap::new() }
    }

    pub fn set(self, key: string, value: i32) {
        self.data.insert(key, value)
    }
}

fn main() {
    let mut store = DataStore::new()
    store.set("score", 42)
}
"#;

    let rust = compile_wj_to_rust(source, "hashmap_insert_owned");

    assert!(
        !rust.is_empty(),
        "Compiler should generate code for HashMap<String> insert with string param"
    );
}

#[test]
fn test_string_param_stored_in_option() {
    let source = r#"
pub struct Timer {
    name: Option<string>,
}

impl Timer {
    pub fn new() -> Timer {
        Timer { name: None }
    }

    pub fn with_name(self, name: string) -> Timer {
        self.name = Some(name)
        self
    }
}

fn main() {
    let mut t = Timer::new()
    t = t.with_name("countdown")
}
"#;

    let rust = compile_wj_to_rust(source, "option_store_owned");

    assert!(
        !rust.is_empty(),
        "Compiler should generate code for Option<String> assignment with string param"
    );
}

#[test]
fn test_string_return_clone_from_string_field() {
    let source = r#"
pub struct Named {
    name: string,
}

impl Named {
    pub fn new(name: string) -> Named {
        Named { name: name }
    }

    pub fn name(self) -> string {
        self.name.clone()
    }
}

fn main() {
    let n = Named::new("test")
    println(n.name())
}
"#;

    let rust = compile_wj_to_rust(source, "return_string_clone");

    assert!(
        !rust.is_empty(),
        "Compiler should generate code for String return from clone()"
    );

    assert!(
        rust.contains("-> String"),
        "Return type string should map to -> String\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_string_param_in_struct_field_init() {
    let source = r#"
pub struct Achievement {
    name: string,
    description: string,
}

impl Achievement {
    pub fn new(name: string, description: string) -> Achievement {
        Achievement {
            name: name,
            description: description,
        }
    }
}

fn main() {
    let a = Achievement::new("first", "Get first achievement")
}
"#;

    let rust = compile_wj_to_rust(source, "struct_field_init_owned");

    assert!(
        !rust.is_empty(),
        "Compiler should generate code for struct init with string params"
    );

    assert!(
        rust.contains("name: String") || rust.contains("name : String"),
        "Struct field 'name' should be String type\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_trait_returns_string_not_str() {
    let source = r#"
pub trait Plugin {
    fn name(self) -> string
    fn version(self) -> string
}

pub struct MyPlugin {}

impl MyPlugin {
    pub fn new() -> MyPlugin {
        MyPlugin {}
    }
}

impl Plugin for MyPlugin {
    fn name(self) -> string {
        "MyPlugin"
    }

    fn version(self) -> string {
        "1.0"
    }
}

fn main() {
    let p = MyPlugin::new()
}
"#;

    let rust = compile_wj_to_rust(source, "trait_string_return");

    assert!(
        !rust.is_empty(),
        "Compiler should generate code for trait with string return"
    );

    assert!(
        rust.contains("-> String"),
        "Trait method returning 'string' should generate '-> String'\nGenerated:\n{}",
        rust
    );
}
