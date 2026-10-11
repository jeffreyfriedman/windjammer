//! Extract test functions from a parsed Windjammer program.
//!
//! A function is a test when it is decorated with `@test` / `@property_test` / `@test_cases`
//! or is named `test_*`. Tests are found at the top level, inside inline `mod name { ... }`
//! blocks (any depth) and as associated functions of non-generic `impl` blocks (with no
//! receiver and no parameters).

use crate::parser::{FunctionDecl, ImplBlock, Item};
use std::path::{Path, PathBuf};

/// Where a test function lives inside its source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TestScope {
    /// Free function at the top level of the file.
    TopLevel,
    /// Free function inside inline modules `mod a { mod b { ... } }`.
    InlineMod(Vec<String>),
    /// Associated function `Type::name()` of an inherent or trait impl.
    Assoc(String),
}

/// Test function metadata.
#[derive(Debug, Clone)]
pub(crate) struct TestFunction {
    pub(crate) name: String,
    pub(crate) file: PathBuf,
    pub(crate) scope: TestScope,
}

impl TestFunction {
    /// Name used in listings and cargo's `module::name` path (relative to the file module).
    pub(crate) fn display_name(&self) -> String {
        match &self.scope {
            TestScope::TopLevel => self.name.clone(),
            TestScope::InlineMod(path) => format!("{}::{}", path.join("::"), self.name),
            TestScope::Assoc(ty) => format!("{ty}::{}", self.name),
        }
    }
}

fn is_test_function(func: &FunctionDecl) -> bool {
    func.name.starts_with("test_")
        || func
            .decorators
            .iter()
            .any(|d| matches!(d.name.as_str(), "test" | "property_test" | "test_cases"))
}

fn runnable_assoc_test(block: &ImplBlock, func: &FunctionDecl) -> bool {
    block.type_params.is_empty() && func.parameters.is_empty() && is_test_function(func)
}

/// Collect every test found in `items` (recursing into inline modules and impl blocks).
pub(crate) fn extract_tests(items: &[Item], file: &Path) -> Vec<TestFunction> {
    let mut out = Vec::new();
    collect(items, file, &mut Vec::new(), &mut out);
    out
}

fn collect(items: &[Item], file: &Path, mod_path: &mut Vec<String>, out: &mut Vec<TestFunction>) {
    for item in items {
        match item {
            Item::Function { decl, .. } if is_test_function(decl) => {
                let scope = if mod_path.is_empty() {
                    TestScope::TopLevel
                } else {
                    TestScope::InlineMod(mod_path.clone())
                };
                out.push(TestFunction {
                    name: decl.name.clone(),
                    file: file.to_path_buf(),
                    scope,
                });
            }
            Item::Impl { block, .. } if mod_path.is_empty() => {
                for func in block.functions.iter().filter(|f| runnable_assoc_test(block, f)) {
                    out.push(TestFunction {
                        name: func.name.clone(),
                        file: file.to_path_buf(),
                        scope: TestScope::Assoc(block.type_name.clone()),
                    });
                }
            }
            Item::Mod { name, items, .. } => {
                mod_path.push(name.clone());
                collect(items, file, mod_path, out);
                mod_path.pop();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn tests_in(source: &str) -> Vec<String> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        extract_tests(&program.items, Path::new("x.wj"))
            .iter()
            .map(TestFunction::display_name)
            .collect()
    }

    #[test]
    fn finds_top_level_decorated_and_prefixed() {
        let src = "@test\nfn checks_it() {}\nfn test_other() {}\nfn helper() {}\n";
        assert_eq!(tests_in(src), vec!["checks_it", "test_other"]);
    }

    #[test]
    fn finds_tests_inside_inline_mods() {
        let src = "mod inner {\n    @test\n    fn deep() {}\n    mod more { fn test_x() {} }\n}\n";
        assert_eq!(tests_in(src), vec!["inner::deep", "inner::more::test_x"]);
    }

    #[test]
    fn finds_receiverless_impl_tests_only() {
        let src = "struct S {}\nimpl S {\n    @test\n    fn test_in_impl() {}\n    fn test_with_arg(x: int) {}\n}\n";
        assert_eq!(tests_in(src), vec!["S::test_in_impl"]);
    }
}
