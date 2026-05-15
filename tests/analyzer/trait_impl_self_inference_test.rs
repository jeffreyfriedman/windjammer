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

// TDD: Trait impl methods must inherit receiver (&self / &mut self) from trait analysis,
// not from impl body alone (empty bodies and missing self in AST must still match trait).

#[path = "../common/test_utils.rs"]
mod test_utils;

// `wj build` is not safe to invoke concurrently from multiple tests (shared cwd / temp paths).

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_inherits_borrowed_self_from_trait() {
    let code = r#"
pub trait Reader {
    fn read() -> string
}

pub struct FileReader {
    path: string
}

impl Reader for FileReader {
    fn read() -> string {
        self.path
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "compile failed: {:?}", result.err());
    let g = result.unwrap();
    assert!(
        g.contains("fn read(&self) -> String"),
        "impl/trait should use &self; got:\n{}",
        g
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_inherits_mut_self_from_trait() {
    let code = r#"
pub trait Counter {
    fn increment()
}

pub struct SimpleCounter {
    count: int
}

impl Counter for SimpleCounter {
    fn increment() {
        self.count = self.count + 1
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "compile failed: {:?}", result.err());
    let g = result.unwrap();
    assert!(
        g.contains("fn increment(&mut self)"),
        "impl/trait should use &mut self; got:\n{}",
        g
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_associated_fn_no_receiver_create_self() {
    let code = r#"
pub trait Factory {
    fn create() -> Self
}

pub struct Thing {
    field: int
}

impl Factory for Thing {
    fn create() -> Thing {
        Thing { field: 0 }
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "compile failed: {:?}", result.err());
    let g = result.unwrap();
    // In Rust, trait definition uses `-> Self`, impl may use `-> Self` or `-> Thing`
    assert!(
        g.contains("fn create() -> Self") || g.contains("fn create() -> Thing"),
        "associated function should have no self receiver; got:\n{}",
        g
    );
    assert!(
        !g.contains("fn create(&self)"),
        "should not infer &self for create() -> Self; got:\n{}",
        g
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_empty_body_still_gets_trait_receiver() {
    let code = r#"
pub trait Port {
    fn shutdown()
}

pub struct Renderer {
    width: int
}

impl Port for Renderer {
    fn shutdown() {
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "compile failed: {:?}", result.err());
    let g = result.unwrap();
    assert!(
        g.contains("fn shutdown(&mut self)"),
        "void abstract trait methods default to &mut self; empty impl must match; got:\n{}",
        g
    );
}
