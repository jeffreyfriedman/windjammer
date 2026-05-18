impl IntInference {
    fn collect_binary_op_constraints<'ast>(
        &mut self,
        left: &'ast Expression<'ast>,
        op: BinaryOp,
        right: &'ast Expression<'ast>,
        return_type: Option<&Type>,
    ) {
        use crate::parser::ast::operators::BinaryOp;
        let left_id = self.get_expr_id(left);
        let right_id = self.get_expr_id(right);

        match op {
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr => {
                // TDD FIX REMOVED: Don't create MustMatch for binary ops!
                // This caused backward propagation:
                //   let n = data.len() as i32  // n is i32
                //   let idx = n / 2  // This creates MustMatch(n, 2)
                //   // Then 2 gets inferred as i32 from context
                //   // OR if written as `n / 2_usize`, MustMatch makes n become usize!
                //
                // Instead, we let each operand keep its declared type and insert casts
                // during code generation when needed.
                //
                // self.constraints.push(IntConstraint::MustMatch(...)); ← REMOVED

                // TDD: items.len() - 1 / items.len() + k → literal must be usize (Rust len is usize)
                if matches!(op, BinaryOp::Add | BinaryOp::Sub) {
                    let left_is_len =
                        matches!(left, Expression::MethodCall { method, .. } if method == "len");
                    let right_is_len =
                        matches!(right, Expression::MethodCall { method, .. } if method == "len");
                    let left_is_literal = matches!(left, Expression::Literal { .. });
                    let right_is_literal = matches!(right, Expression::Literal { .. });
                    if left_is_len && right_is_literal {
                        self.constraints.push(IntConstraint::MustBe(
                            right_id,
                            IntType::Usize,
                            "arithmetic with .len() (usize)".to_string(),
                        ));
                    }
                    if right_is_len && left_is_literal {
                        self.constraints.push(IntConstraint::MustBe(
                            left_id,
                            IntType::Usize,
                            "arithmetic with .len() (usize)".to_string(),
                        ));
                    }
                }

                // TDD FIX: Don't propagate types bidirectionally in binary ops.
                // Each side keeps its own type. Code generation will insert casts.
                // (Lines 54-144 REMOVED)
            }
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                // TDD FIX REMOVED: Don't create MustMatch for comparisons!
                // Same issue as arithmetic - causes backward propagation.
                // Code generation will insert casts when types don't match.
                //
                // self.constraints.push(IntConstraint::MustMatch(...)); ← REMOVED

                // TDD FIX: When comparing with .len() (returns usize), constrain literal to usize
                // e.g., items.len() > 0 → 0 should be usize, not i32
                let left_is_len =
                    matches!(left, Expression::MethodCall { method, .. } if method == "len");
                let right_is_len =
                    matches!(right, Expression::MethodCall { method, .. } if method == "len");
                let left_is_literal = matches!(left, Expression::Literal { .. });
                let right_is_literal = matches!(left, Expression::Literal { .. });

                if left_is_len && right_is_literal {
                    self.constraints.push(IntConstraint::MustBe(
                        right_id,
                        IntType::Usize,
                        "comparison with .len() (usize)".to_string(),
                    ));
                }
                if right_is_len && left_is_literal {
                    self.constraints.push(IntConstraint::MustBe(
                        left_id,
                        IntType::Usize,
                        "comparison with .len() (usize)".to_string(),
                    ));
                }

                // TDD FIX: Don't propagate types in comparisons.
                // Each side keeps its declared type. Code generation handles type conversion.
                // (Lines 182-272 REMOVED)
            }
            _ => {}
        }

        self.collect_expression_constraints(left, return_type);
        self.collect_expression_constraints(right, return_type);
    }
}
