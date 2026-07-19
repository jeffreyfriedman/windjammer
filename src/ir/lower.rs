//! AST → expression-level IR lowering.
//!
//! Builds `IrNode` trees with placeholder safety types. The constraint solver
//! and pipeline write-back populate authoritative `SafetyType` data.

use crate::ir::node::{IrNode, IrNodeKind};
use crate::ir::safety_type::{BaseType, SafetyType};
use crate::parser::{Expression, Literal, Pattern, Statement};

/// Lower a function body to IR nodes (expression-level skeleton).
pub fn lower_body(body: &[&Statement]) -> Vec<IrNode> {
    body.iter()
        .flat_map(|stmt| lower_statement(stmt))
        .collect()
}

fn lower_statement(stmt: &Statement) -> Vec<IrNode> {
    match stmt {
        Statement::Let { pattern, mutable, .. } => {
            let name = pattern_binding_name(pattern).unwrap_or_else(|| "_".to_string());
            vec![IrNode {
                kind: IrNodeKind::Let {
                    name,
                    mutable: *mutable,
                },
                safety_type: SafetyType::owned(BaseType::Inferred),
            }]
        }
        Statement::Expression { expr, .. } => vec![lower_expression(expr)],
        Statement::Return { value, .. } => value
            .as_ref()
            .map(|e| vec![lower_expression(e)])
            .unwrap_or_default(),
        Statement::If { then_block, else_block, .. } => {
            let mut nodes = lower_block(then_block);
            if let Some(else_b) = else_block {
                nodes.extend(lower_block(else_b));
            }
            nodes
        }
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Loop { body, .. }
        | Statement::Thread { body, .. }
        | Statement::Async { body, .. } => lower_block(body),
        Statement::Assignment { value, .. } => vec![lower_expression(value)],
        _ => Vec::new(),
    }
}

fn lower_block(block: &[&Statement]) -> Vec<IrNode> {
    block.iter().flat_map(|stmt| lower_statement(stmt)).collect()
}

fn pattern_binding_name(pattern: &Pattern) -> Option<String> {
    match pattern {
        Pattern::Identifier(name) => Some(name.clone()),
        Pattern::Wildcard => None,
        _ => None,
    }
}

fn lower_expression(expr: &Expression) -> IrNode {
    match expr {
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            let callee_name = match function {
                Expression::Identifier { name, .. } => name.to_string(),
                Expression::FieldAccess { field, .. } => field.clone(),
                _ => "<expr>".to_string(),
            };
            IrNode {
                kind: IrNodeKind::Call {
                    callee: callee_name,
                    args: arguments
                        .iter()
                        .map(|(_label, arg)| lower_expression(arg))
                        .collect(),
                },
                safety_type: SafetyType::owned(BaseType::Inferred),
            }
        }
        Expression::MethodCall {
            method,
            arguments,
            ..
        } => IrNode {
            kind: IrNodeKind::Call {
                callee: method.clone(),
                args: arguments
                    .iter()
                    .map(|(_label, arg)| lower_expression(arg))
                    .collect(),
            },
            safety_type: SafetyType::owned(BaseType::Inferred),
        },
        Expression::FieldAccess { object, field, .. } => IrNode {
            kind: IrNodeKind::FieldAccess {
                base: Box::new(lower_expression(object)),
                field: field.clone(),
            },
            safety_type: SafetyType::owned(BaseType::Inferred),
        },
        Expression::Literal { value, .. } => {
            let base = match value {
                Literal::Int(_) => BaseType::I32,
                Literal::Float(_) => BaseType::F64,
                Literal::Bool(_) => BaseType::Bool,
                Literal::String(_) => BaseType::String,
                _ => BaseType::Inferred,
            };
            IrNode {
                kind: IrNodeKind::AstPassthrough,
                safety_type: SafetyType::owned(base),
            }
        }
        _ => IrNode {
            kind: IrNodeKind::AstPassthrough,
            safety_type: SafetyType::owned(BaseType::Inferred),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::parse_wj_source;
    use crate::parser::Item;
    use std::path::Path;

    #[test]
    fn lower_call_produces_call_node() {
        let (_parser, program) =
            parse_wj_source(Path::new("test.wj"), "fn f() { g(1) }").expect("parse");
        let func = match &program.items[0] {
            Item::Function { decl: f, .. } => f,
            _ => panic!("expected function"),
        };
        let nodes = lower_body(&func.body);
        assert!(nodes.iter().any(|n| matches!(n.kind, IrNodeKind::Call { .. })));
    }
}
