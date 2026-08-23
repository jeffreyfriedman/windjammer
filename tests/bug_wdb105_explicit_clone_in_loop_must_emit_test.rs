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

//! WDB-105: explicit `.clone()` on loop-carried non-Copy values passed to owned
//! trait method formals must appear in generated Rust (Phase 153 grammar extensions).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn wdb105_explicit_clone_in_while_loop_trait_call_must_emit() {
    let source = r#"
pub enum Ast {
    Select(i64),
    Sum(i64),
}

pub struct Cursor {
    pub pos: i64,
}

pub trait SelectAtomExtension {
    fn try_apply(self, ast: Ast, cursor: Cursor) -> Option<(Ast, Cursor)>;
}

pub struct LimitExt {
    pub enabled: bool,
}

impl SelectAtomExtension for LimitExt {
    fn try_apply(self, ast: Ast, cursor: Cursor) -> Option<(Ast, Cursor)> {
        if !self.enabled {
            return None
        }
        Some((ast, cursor))
    }
}

pub fn apply_extensions(ast: Ast, cursor: Cursor) -> (Ast, Cursor) {
    let mut current_ast = ast
    let mut current = cursor
    let mut guard = 0
    while guard < 8 {
        guard = guard + 1
        let ext = LimitExt { enabled: true }
        match ext.try_apply(current_ast.clone(), current.clone()) {
            Some(pair) => {
                current_ast = pair.0
                current = pair.1
            },
            None => {
                break
            },
        }
    }
    (current_ast, current)
}
"#;

    let rs = test_utils::compile_single(source);
    assert!(
        rs.contains("current_ast.clone()") && rs.contains("current.clone()"),
        "WDB-105: explicit clone in loop must be preserved in codegen. Got:\n{rs}"
    );
}
