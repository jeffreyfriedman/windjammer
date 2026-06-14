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

#[path = "common/test_utils.rs"]
mod test_utils;

/// TDD Test: TryOp (?) Ownership Inference
///
/// PROBLEM: When a parameter is used inside a `?` (try/error propagation) expression,
/// the analyzer fails to detect the usage because `Expression::TryOp` wraps the inner
/// expression and the walking functions don't recurse into it.
///
/// Example:
/// ```
/// pub fn load_game_level(loader: AssetLoader, level_name: String) -> Result<...> {
///     let tilemap = loader.load("tilemap", "path", 8192)?
/// }
/// ```
///
/// The AST for `loader.load(...)?` is:
///   TryOp { expr: MethodCall { object: Identifier("loader"), method: "load", ... } }
///
/// The analyzer sees TryOp and returns false (catch-all), never checking the inner
/// MethodCall. This causes `loader` to be inferred as `&AssetLoader` (borrowed)
/// when it should stay Owned (since .load() is potentially mutating).
///
/// FIX: Add TryOp handling to all walking functions in the analyzer so that
/// expressions wrapped in `?` are still analyzed for ownership inference.
#[test]
fn test_tryop_method_call_keeps_param_owned() {
    // loader.load(...)? — .load() is potentially mutating, so loader should stay Owned
    let source = r#"
struct AssetLoader {
    pub base_path: string,
}

impl AssetLoader {
    fn new(path: string) -> AssetLoader {
        AssetLoader { base_path: path }
    }

    fn load(self, name: string) -> Result<string, string> {
        Ok(name)
    }
}

fn load_game(loader: AssetLoader) -> Result<string, string> {
    let result = loader.load("tilemap".to_string())?
    Ok(result)
}
"#;

    let rust_code = test_utils::compile_single(source);

    // The parameter should NOT be borrowed because .load() is potentially mutating
    // and it's inside a TryOp expression
    assert!(
        !rust_code.contains("loader: &AssetLoader"),
        "loader should NOT be &AssetLoader (borrowed) when .load()? is called.\n\
         The ? operator wraps the method call in TryOp, which must be recursed into.\n\
         Generated:\n{}",
        rust_code
    );

    // It should be either owned or mut borrowed
    let has_owned =
        rust_code.contains("loader: AssetLoader") || rust_code.contains("mut loader: AssetLoader");
    let has_mut_ref = rust_code.contains("loader: &mut AssetLoader");
    assert!(
        has_owned || has_mut_ref,
        "loader should be AssetLoader (owned) or &mut AssetLoader since .load() needs &mut self.\n\
         Generated:\n{}",
        rust_code
    );
}

#[test]
fn test_tryop_passed_as_argument_keeps_param_owned() {
    // process(data)? — data is passed as argument to a function that returns Result
    // Use a non-String, non-Copy custom type to properly test TryOp handling
    // Payload has a Vec<i32> field so it's definitely not Copy
    let source = r#"
struct Payload {
    pub items: Vec<i32>,
}

fn process(data: Payload) -> Result<i32, string> {
    Ok(data.items.len() as i32)
}

fn run(data: Payload) -> Result<i32, string> {
    let result = process(data)?
    Ok(result)
}
"#;

    let rust_code = test_utils::compile_single(source);

    // data is passed as an argument to process() inside a TryOp,
    // so it should stay owned (consumed by the call)
    assert!(
        !rust_code.contains("fn run(data: &Payload)"),
        "data should NOT be &Payload when passed to process(data)?.\n\
         Generated:\n{}",
        rust_code
    );
}
