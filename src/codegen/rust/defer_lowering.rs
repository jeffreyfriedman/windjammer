//! `defer stmt` lowering (interpreter semantics: deferred statements run at function exit, last-in first-out,
//! also when the function panics).
//!
//! Each `defer` becomes a scope guard local whose `Drop` runs the statement. Rust drops locals in reverse
//! declaration order, so several defers run LIFO, and `Drop` runs during unwinding (panic-safe). The guard lives
//! in the enclosing block, so a `defer` nested inside a block (not at the top level of the function body) would
//! run at that block's end instead: [`unsupported_constructs`] reports it so `wj check` can reject it.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{FunctionDecl, Item, Program, Statement};

/// A construct the Rust backend cannot lower with the interpreter's semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedConstruct {
    pub function: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_defer_statement(
        &mut self,
        inner: &'ast Statement<'ast>,
        whole: &Statement<'ast>,
    ) -> String {
        let id = whole.location().map_or(0, |l| l.line * 1000 + l.column);
        let pad = self.indent();
        self.indent_level += 2;
        let body = self.generate_statement(inner);
        self.indent_level -= 2;
        format!(
            "{pad}let _wj_defer_{id} = {{\n{pad}    struct WjDefer<F: FnMut()>(F);\n{pad}    impl<F: FnMut()> Drop for WjDefer<F> {{\n{pad}        fn drop(&mut self) {{\n{pad}            (self.0)()\n{pad}        }}\n{pad}    }}\n{pad}    WjDefer(|| {{\n{body}{pad}    }})\n{pad}}};\n"
        )
    }
}

fn nested_defers(stmts: &[&Statement], top: bool, f: &FunctionDecl, out: &mut Vec<UnsupportedConstruct>) {
    for s in stmts {
        match s {
            Statement::Defer { location, .. } if !top => out.push(UnsupportedConstruct {
                function: f.name.clone(),
                line: location.as_ref().map_or(0, |l| l.line),
                column: location.as_ref().map_or(0, |l| l.column),
                message: "`defer` inside a nested block: compiled code would run it at the end of that block, the \
                          interpreter at function exit; move it to the top level of the function body"
                    .to_string(),
            }),
            Statement::If { then_block, else_block, .. } => {
                nested_defers(then_block, false, f, out);
                if let Some(e) = else_block {
                    nested_defers(e, false, f, out);
                }
            }
            Statement::For { body, .. }
            | Statement::While { body, .. }
            | Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => nested_defers(body, false, f, out),
            _ => {}
        }
    }
}

fn collect_item(item: &Item, out: &mut Vec<UnsupportedConstruct>) {
    match item {
        Item::Function { decl, .. } => nested_defers(&decl.body, true, decl, out),
        Item::Impl { block, .. } => block.functions.iter().for_each(|f| nested_defers(&f.body, true, f, out)),
        Item::Mod { items, .. } => items.iter().for_each(|i| collect_item(i, out)),
        _ => {}
    }
}

/// Constructs in `program` the compiled (Rust) path cannot lower with interpreter semantics. Library entry point
/// for `wj check`: a non-empty result means the program must not be reported as checked-OK.
pub fn unsupported_constructs(program: &Program) -> Vec<UnsupportedConstruct> {
    let mut out = Vec::new();
    program.items.iter().for_each(|i| collect_item(i, &mut out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Program<'static> {
        let tokens = crate::lexer::Lexer::new(src).tokenize_with_locations();
        crate::parser::Parser::new_with_source(tokens, "t.wj".to_string(), src.to_string()).parse().unwrap()
    }

    #[test]
    fn top_level_defer_is_supported_nested_is_reported() {
        assert!(unsupported_constructs(&parse("fn main() {\n    defer println(\"a\")\n}\n")).is_empty());
        let r = unsupported_constructs(&parse("fn main() {\n    if true {\n        defer println(\"a\")\n    }\n}\n"));
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].function, "main");
    }
}
