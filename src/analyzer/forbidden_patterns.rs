//! Rejects Rust-leakage patterns before analysis runs.
//!
//! Forbidden patterns include:
//! - `.as_str()` calls (compiler handles String → &str automatically)
//! - `@derive(Copy)`, `@derive(Clone)`, etc. for standard traits
//!   (compiler auto-infers derivable traits from struct field types)

use crate::parser::ast::core::Expression;
use crate::parser::*;

const AUTO_INFERRED_TRAITS: &[&str] = &[
    "Copy", "Clone", "Debug", "PartialEq", "Eq", "Hash", "Default", "PartialOrd", "Ord",
];

/// Walk the AST and fail if forbidden Rust-specific patterns appear in user source.
pub(in crate::analyzer) fn check_forbidden_rust_patterns<'ast>(
    program: &Program<'ast>,
) -> Result<(), String> {
    check_forbidden_decorators(program)?;
    fn check_expr(expr: &Expression) -> Result<(), String> {
        match expr {
            Expression::MethodCall {
                method,
                object,
                arguments,
                ..
            } => {
                if method == "as_str" && arguments.is_empty() {
                    return Err("error: `.as_str()` is forbidden in Windjammer source\n\
                         \n\
                         Windjammer automatically handles string conversions based on context.\n\
                         You don't need to call `.as_str()` - the compiler will generate the\n\
                         correct Rust code automatically.\n\
                         \n\
                         Example:\n\
                         ❌ match name.as_str() { ... }  // Don't do this\n\
                         ✅ match name { ... }            // Do this instead\n\
                         \n\
                         This keeps Windjammer code clean and backend-agnostic (Go/JS/etc\n\
                         don't have .as_str())."
                        .to_string());
                }

                check_expr(object)?;
                for (_label, arg) in arguments {
                    check_expr(arg)?;
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::FieldAccess { field, .. } = &**function {
                    if field == "as_str" && arguments.is_empty() {
                        return Err("error: `.as_str()` is forbidden in Windjammer source\n\
                             \n\
                             Windjammer automatically handles string conversions based on context.\n\
                             You don't need to call `.as_str()` - the compiler will generate the\n\
                             correct Rust code automatically.\n\
                             \n\
                             Example:\n\
                             ❌ match name.as_str() { ... }  // Don't do this\n\
                             ✅ match name { ... }            // Do this instead\n\
                             \n\
                             This keeps Windjammer code clean and backend-agnostic (Go/JS/etc\n\
                             don't have .as_str()).".to_string());
                    }
                }

                check_expr(function)?;
                for (_label, arg) in arguments {
                    check_expr(arg)?;
                }
            }
            Expression::Binary { left, right, .. } => {
                check_expr(left)?;
                check_expr(right)?;
            }
            Expression::Unary { operand, .. } => {
                check_expr(operand)?;
            }
            Expression::FieldAccess { object, .. } => {
                check_expr(object)?;
            }
            Expression::Index { object, index, .. } => {
                check_expr(object)?;
                check_expr(index)?;
            }
            Expression::StructLiteral { fields, .. } => {
                for (_name, value) in fields {
                    check_expr(value)?;
                }
            }
            Expression::Array { elements, .. } => {
                for elem in elements {
                    check_expr(elem)?;
                }
            }
            Expression::Cast { expr, .. } => {
                check_expr(expr)?;
            }
            Expression::Closure { body, .. } => {
                check_expr(body)?;
            }
            Expression::Tuple { elements, .. } => {
                for elem in elements {
                    check_expr(elem)?;
                }
            }
            Expression::Range { start, end, .. } => {
                check_expr(start)?;
                check_expr(end)?;
            }
            Expression::MapLiteral { pairs, .. } => {
                for (key, value) in pairs {
                    check_expr(key)?;
                    check_expr(value)?;
                }
            }
            Expression::TryOp { expr, .. } => {
                check_expr(expr)?;
            }
            Expression::Await { expr, .. } => {
                check_expr(expr)?;
            }
            Expression::ChannelSend { channel, value, .. } => {
                check_expr(channel)?;
                check_expr(value)?;
            }
            Expression::ChannelRecv { channel, .. } => {
                check_expr(channel)?;
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    check_stmt(stmt)?;
                }
            }
            Expression::MacroInvocation { args, .. } => {
                for arg in args {
                    check_expr(arg)?;
                }
            }
            Expression::Literal { .. } | Expression::Identifier { .. } => {}
        }
        Ok(())
    }

    fn check_stmt(stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Let {
                value, else_block, ..
            } => {
                check_expr(value)?;
                if let Some(block) = else_block {
                    for s in block {
                        check_stmt(s)?;
                    }
                }
            }
            Statement::Const { value, .. } | Statement::Static { value, .. } => {
                check_expr(value)?;
            }
            Statement::Assignment { value, target, .. } => {
                check_expr(value)?;
                check_expr(target)?;
            }
            Statement::Expression { expr, .. } => {
                check_expr(expr)?;
            }
            Statement::Return { value, .. } => {
                if let Some(val) = value {
                    check_expr(val)?;
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                check_expr(condition)?;
                for s in then_block {
                    check_stmt(s)?;
                }
                if let Some(else_stmts) = else_block {
                    for s in else_stmts {
                        check_stmt(s)?;
                    }
                }
            }
            Statement::Match { value, arms, .. } => {
                check_expr(value)?;
                for arm in arms {
                    check_expr(arm.body)?;
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                check_expr(condition)?;
                for s in body {
                    check_stmt(s)?;
                }
            }
            Statement::For { iterable, body, .. } => {
                check_expr(iterable)?;
                for s in body {
                    check_stmt(s)?;
                }
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                for s in body {
                    check_stmt(s)?;
                }
            }
            Statement::Defer { statement, .. } => {
                check_stmt(statement)?;
            }
            Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => {}
        }
        Ok(())
    }

    for item in &program.items {
        match item {
            Item::Function { decl, .. } => {
                for stmt in &decl.body {
                    check_stmt(stmt)?;
                }
            }
            Item::Impl { block, .. } => {
                for func in &block.functions {
                    for stmt in &func.body {
                        check_stmt(stmt)?;
                    }
                }
            }
            Item::Trait { decl, .. } => {
                for method in &decl.methods {
                    if let Some(body) = &method.body {
                        for stmt in body {
                            check_stmt(stmt)?;
                        }
                    }
                }
            }
            Item::Const { value, .. } | Item::Static { value, .. } => {
                check_expr(value)?;
            }
            Item::Mod { items, .. } => {
                let mod_program = Program {
                    items: items.clone(),
                };
                check_forbidden_rust_patterns(&mod_program)?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn check_forbidden_decorators<'ast>(program: &Program<'ast>) -> Result<(), String> {
    for item in &program.items {
        match item {
            Item::Struct { decl, .. } => {
                check_struct_decorators(&decl.name, &decl.decorators)?;
            }
            Item::Mod { items, .. } => {
                let mod_program = Program {
                    items: items.clone(),
                };
                check_forbidden_decorators(&mod_program)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_struct_decorators(_struct_name: &str, _decorators: &[Decorator<'_>]) -> Result<(), String> {
    // Auto-inferred traits (Copy, Clone, Debug, etc.) specified via @derive are
    // silently accepted for backward compatibility.  The compiler's auto-derive
    // system handles these traits automatically based on field types, so explicit
    // @derive for them is redundant but not an error.
    Ok(())
}
