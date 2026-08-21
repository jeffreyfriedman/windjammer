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

/// TDD: TryOp (`?`) must recurse into the inner expression for ownership.
///
/// Signature-driven: when the method under `?` takes owned/`&mut self`, the
/// param stays owned (or mut-borrowed). When the method is clearly `&self`,
/// borrowing the param is correct — do not poison via unqualified method-name
/// consensus (`load` is not a stdlib-wide mutate oracle).

#[test]
fn test_tryop_readonly_method_may_borrow_param() {
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

    // Body does not use `self` fields → analyzer converges `&self` → call-site borrow OK.
    assert!(
        rust_code.contains("loader: &AssetLoader")
            || rust_code.contains("fn load(&self"),
        "readonly AssetLoader::load under ? may borrow loader.\nGenerated:\n{rust_code}"
    );
    assert!(
        !rust_code.contains("loader: &mut AssetLoader"),
        "readonly load must not force &mut.\nGenerated:\n{rust_code}"
    );
}

#[test]
fn test_tryop_mutating_method_keeps_param_mut_or_owned() {
    // Field mutation under `?` must not demote the param to shared `&T`.
    let source = r#"
struct AssetLoader {
    pub base_path: string,
}

impl AssetLoader {
    fn load(self, name: string) -> Result<string, string> {
        self.base_path = name
        Ok(self.base_path)
    }
}

fn load_game(loader: AssetLoader) -> Result<string, string> {
    let result = loader.load("tilemap".to_string())?
    Ok(result)
}
"#;

    let rust_code = test_utils::compile_single(source);

    assert!(
        !rust_code.contains("loader: &AssetLoader"),
        "mutating load()? must not shared-borrow loader.\nGenerated:\n{rust_code}"
    );
    let has_owned =
        rust_code.contains("loader: AssetLoader") || rust_code.contains("mut loader: AssetLoader");
    let has_mut_ref = rust_code.contains("loader: &mut AssetLoader");
    assert!(
        has_owned || has_mut_ref,
        "mutating method under ? must keep owned or &mut formal.\nGenerated:\n{rust_code}"
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
