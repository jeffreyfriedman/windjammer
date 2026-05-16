//! Operator and type conversion expression generation
//!
//! Handles generation of:
//! - Type casts (expr as Type)
//! - Unary operators (!expr, -expr, *expr, &expr, &mut expr)

use crate::parser::{Expression, OwnershipHint, Type};

use super::{operators, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for cast expression (expr as Type)
    /// E0606 FIX: Cannot cast &T as U - auto-deref borrowed parameters first
    pub(in crate::codegen::rust) fn generate_cast(
        &mut self,
        expr: &Expression<'ast>,
        type_: &Type,
    ) -> String {
        // Add parentheses around binary expressions for correct precedence
        // because `as` has higher precedence than arithmetic in Rust:
        // `a + b as usize` is parsed as `a + (b as usize)`, not `(a + b) as usize`
        let mut expr_str = match expr {
            Expression::Binary { .. } => {
                format!("({})", self.generate_expression(expr))
            }
            _ => self.generate_expression(expr),
        };
        // E0606 FIX: Cannot cast &T as U (e.g. &i32 as usize).
        // When the cast source is a borrowed parameter, auto-deref first.
        if let Expression::Identifier { name, .. } = expr {
            let is_borrowed_param = self.inferred_borrowed_params.contains(name)
                || self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && matches!(p.ownership, OwnershipHint::Ref | OwnershipHint::Mut)
                });
            if is_borrowed_param && !expr_str.starts_with('*') {
                expr_str = format!("*{}", expr_str);
            }
        }
        let type_str = self.type_to_rust(type_);
        format!("{} as {}", expr_str, type_str)
    }

    /// Generate code for unary expression (!expr, -expr, *expr, &expr, &mut expr)
    pub(in crate::codegen::rust) fn generate_unary(
        &mut self,
        op: &crate::parser::UnaryOp,
        operand: &Expression<'ast>,
    ) -> String {
        use crate::parser::Literal;
        if matches!(op, crate::parser::UnaryOp::Neg) {
            if let Expression::Literal {
                value: lit @ Literal::Int(_),
                ..
            } = operand
            {
                let s = self.generate_literal_with_context(lit, operand);
                return format!("-{}", s);
            }
        }

        // TDD FIX: Explicit deref handling is now in balance_eq_operands_for_rust
        // where we have access to BOTH operands to make the right decision

        // Strip `*` for owned Copy types that don't implement Deref.
        // User writes `*id` but `id` is already owned (e.g., from for-loop over owned Vec) —
        // dereffing a non-Deref type is E0614.
        // BUT keep `*` when the variable is actually a reference (local ref, borrowed param,
        // borrowed iterator var) — deref is valid and necessary there.
        if matches!(op, crate::parser::UnaryOp::Deref) {
            if let Expression::Identifier { name, .. } = operand {
                let is_borrowed = self.inferred_borrowed_params.contains(name.as_str())
                    || self.borrowed_iterator_vars.contains(name);
                let is_local_ref = self.local_var_types.get(name.as_str()).is_some_and(|t| {
                    matches!(
                        t,
                        crate::parser::Type::Reference(_)
                            | crate::parser::Type::MutableReference(_)
                    )
                });
                if !is_borrowed && !is_local_ref {
                    let is_copy = self
                        .infer_expression_type(operand)
                        .as_ref()
                        .is_some_and(|t| self.is_type_copy(t));
                    if is_copy {
                        return self.generate_expression(operand);
                    }
                }
            }
        }

        let op_str = operators::unary_op_to_rust(op);

        // BORROW CONTEXT: When generating &expr or &mut expr, suppress Vec index
        // auto-clone in the operand. We want a reference to the original element.
        // e.g., &self.items[i] → NOT &self.items[i].clone()
        //        &mut self.items[i] → NOT &mut self.items[i].clone()
        let is_borrow = matches!(
            op,
            crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef
        );
        let is_deref = matches!(op, crate::parser::UnaryOp::Deref);
        let prev_borrow = self.in_borrow_context;
        if is_borrow {
            self.in_borrow_context = true;
        }
        // When generating *expr, suppress in_owned_value_context for the
        // inner operand to prevent double-deref (**x). The explicit * already
        // handles the deref; the owned-value-context * would be redundant.
        let prev_owned = self.in_owned_value_context;
        if is_deref {
            self.in_owned_value_context = false;
        }
        let operand_str = self.generate_expression(operand);
        self.in_borrow_context = prev_borrow;
        self.in_owned_value_context = prev_owned;

        // CRITICAL: Preserve parentheses for binary expressions in unary context
        // !(a || b) should generate !(a || b), not !a || b
        // Binary operators have lower precedence than unary operators, so we need parens
        let needs_parens = matches!(operand, Expression::Binary { .. });

        if needs_parens {
            format!("{}({})", op_str, operand_str)
        } else {
            format!("{}{}", op_str, operand_str)
        }
    }
}
