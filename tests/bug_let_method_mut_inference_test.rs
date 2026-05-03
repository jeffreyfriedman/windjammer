#[path = "test_utils.rs"]
mod test_utils;

/// TDD test: Mutable method calls in let bindings should trigger &mut inference
///
/// Bug: `let x = loader.load(...)` where `load()` requires `&mut self`
/// doesn't trigger &mut inference for `loader` parameter because `is_mutated`
/// only checks `Statement::Expression`, not `Statement::Let` values.
///
/// Root Cause: `is_mutated` doesn't check the value expression of let bindings
/// for mutable method calls.
///
/// Fix: Add `Statement::Let` case in `is_mutated` to check value for
/// mutable method calls.
#[test]
fn test_let_binding_with_mut_method_call() {
    let source = r#"
pub struct Loader {
    count: i32,
    items: Vec<String>,
}

impl Loader {
    pub fn new() -> Loader {
        Loader { count: 0, items: Vec::new() }
    }

    pub fn load(self, name: String, size: i32) -> String {
        self.count = self.count + 1
        self.items.push(name.clone())
        name
    }
}

pub fn load_stuff(loader: Loader) -> Vec<String> {
    let mut results: Vec<String> = Vec::new()
    let a = loader.load("first".to_string(), 100)
    let b = loader.load("second".to_string(), 200)
    results.push(a)
    results.push(b)
    results
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    // THE WINDJAMMER WAY: Automatic ownership inference!
    // User writes `loader: Loader` (no & or &mut)
    // Compiler infers `loader: &mut Loader` because loader.load() mutates (self.count++)
    // This is automatic ownership inference - compiler does the hard work!
    assert!(
        generated.contains("loader: &mut Loader"),
        "Parameter should be inferred as `&mut Loader` (automatic ownership). Got:\n{}",
        generated
    );
}
