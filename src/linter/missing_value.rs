//! W0010 (ADR-0015 #4): a function that declares a return type must end in a value on every path.
//!
//! Windjammer uses an implicit final expression, so the classic mistakes are a body whose last
//! statement is not a value (`let`, assignment, loop, `if` without `else`, a `println(..)` call) or
//! a path that falls off the end. The analysis is conservative: it only reports when the last
//! statement is definitely not a value, so it never rejects a correct program.

use crate::parser::ast::core::{Expression, FunctionDecl, Item, Program, Statement};
use crate::parser::ast::types::Type;
use crate::source_map::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingValue {
    pub function: String,
    pub location: Option<Location>,
    pub reason: String,
}

fn declares_value(ret: &Option<Type>) -> bool {
    match ret {
        None => false,
        Some(Type::Tuple(v)) => !v.is_empty(),
        Some(Type::Custom(n)) => !matches!(n.as_str(), "()" | "Unit" | "unit" | "!" | "Never"),
        Some(_) => true,
    }
}

/// `Some(reason)` when `block` definitely does not end in a value.
fn no_value(block: &[&Statement]) -> Option<String> {
    let Some(last) = block.last() else {
        return Some("the body is empty".to_string());
    };
    match last {
        Statement::Let { .. } | Statement::Const { .. } | Statement::Static { .. } => {
            Some("the last statement is a `let`, not a value".to_string())
        }
        Statement::Assignment { .. } => Some("the last statement is an assignment, which has no value".to_string()),
        Statement::For { .. } | Statement::While { .. } => {
            Some("the last statement is a loop that produces no value".to_string())
        }
        Statement::If { then_block, else_block, .. } => match else_block {
            None => Some("the final `if` has no `else`, so the false path produces no value".to_string()),
            Some(e) => no_value(then_block).or_else(|| no_value(e)),
        },
        Statement::Expression { expr, .. } => unit_expression(expr),
        Statement::Match { arms, .. } => arms.iter().find_map(|a| match a.body {
            Expression::Block { statements, .. } => no_value(statements),
            other => unit_expression(other),
        }),
        _ => None,
    }
}

/// Calls whose result is always `()`.
fn unit_expression(e: &Expression) -> Option<String> {
    if let Expression::Call { function, .. } = e {
        if let Expression::Identifier { name, .. } = function {
            if matches!(name.as_str(), "println" | "print" | "eprintln" | "eprint") {
                return Some(format!("the last expression is `{name}(..)`, which is `()`"));
            }
        }
    }
    None
}

fn check_function(f: &FunctionDecl, out: &mut Vec<MissingValue>) {
    if f.is_extern || !declares_value(&f.return_type) || f.body.is_empty() {
        return;
    }
    if let Some(reason) = no_value(&f.body) {
        let location = f.body.last().and_then(|s| s.location());
        out.push(MissingValue { function: f.name.clone(), location, reason });
    }
}

fn check_item(item: &Item, out: &mut Vec<MissingValue>) {
    match item {
        Item::Function { decl, .. } => check_function(decl, out),
        Item::Impl { block, .. } => block.functions.iter().for_each(|f| check_function(f, out)),
        Item::Mod { items, .. } => items.iter().for_each(|i| check_item(i, out)),
        _ => {}
    }
}

/// Every function that declares a return type but definitely ends without a value.
pub fn missing_values(program: &Program) -> Vec<MissingValue> {
    let mut out = Vec::new();
    program.items.iter().for_each(|i| check_item(i, &mut out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> Vec<MissingValue> {
        let tokens = crate::lexer::Lexer::new(src).tokenize_with_locations();
        let mut p = crate::parser::Parser::new_with_source(tokens, "t.wj".to_string(), src.to_string());
        missing_values(&p.parse().unwrap())
    }

    #[test]
    fn last_let_is_reported() {
        let r = run("fn f(x: int) -> int {\n    let y = x + 1\n}\n");
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].reason.contains("let"));
    }

    #[test]
    fn if_without_else_is_reported() {
        assert_eq!(run("fn f(x: int) -> int {\n    if x > 1 {\n        return 1\n    }\n}\n").len(), 1);
    }

    #[test]
    fn println_last_is_reported() {
        assert_eq!(run("fn g() -> int {\n    println(\"hi\")\n}\n").len(), 1);
    }

    #[test]
    fn valid_shapes_are_accepted() {
        for src in [
            "fn f(x: int) -> int {\n    x + 1\n}\n",
            "fn f(x: int) -> int {\n    return x\n}\n",
            "fn f(x: int) -> int {\n    if x > 1 {\n        1\n    } else {\n        2\n    }\n}\n",
            "fn f(x: int) -> int {\n    match x {\n        1 => 10,\n        _ => 20,\n    }\n}\n",
            "fn f(x: int) -> int {\n    loop {\n        return x\n    }\n}\n",
            "fn f() {\n    let y = 1\n}\n",
            "fn f() -> int {\n    panic(\"no\")\n}\n",
        ] {
            assert!(run(src).is_empty(), "false positive for:\n{src}");
        }
    }
}
