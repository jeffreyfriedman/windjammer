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

        let left_is_literal = matches!(left, Expression::Literal { value: crate::parser::Literal::Int(_), .. });
        let right_is_literal = matches!(right, Expression::Literal { value: crate::parser::Literal::Int(_), .. });

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
                if matches!(op, BinaryOp::Add | BinaryOp::Sub) {
                    let left_is_len =
                        matches!(left, Expression::MethodCall { method, .. } if method == "len");
                    let right_is_len =
                        matches!(right, Expression::MethodCall { method, .. } if method == "len");
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

                self.propagate_typed_operand_to_literal(left, right, left_id, right_id, left_is_literal, right_is_literal, "arithmetic");
            }
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                let left_is_len =
                    matches!(left, Expression::MethodCall { method, .. } if method == "len");
                let right_is_len =
                    matches!(right, Expression::MethodCall { method, .. } if method == "len");
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

                self.propagate_typed_operand_to_literal(left, right, left_id, right_id, left_is_literal, right_is_literal, "comparison");
            }
            _ => {}
        }

        self.collect_expression_constraints(left, return_type);
        self.collect_expression_constraints(right, return_type);
    }

    /// One-directional propagation: when one operand of a binary op is a typed
    /// expression (field access, variable, etc.) and the other is an unsuffixed
    /// int literal, constrain the literal to match the typed operand's type.
    /// This avoids the bidirectional MustMatch that caused backward propagation bugs.
    #[allow(clippy::too_many_arguments)]
    fn propagate_typed_operand_to_literal<'ast>(
        &mut self,
        left: &'ast Expression<'ast>,
        right: &'ast Expression<'ast>,
        left_id: ExprId,
        right_id: ExprId,
        left_is_literal: bool,
        right_is_literal: bool,
        _context: &str,
    ) {
        if right_is_literal && !left_is_literal {
            if let Some(int_type) = self.resolve_expression_int_type(left) {
                self.constraints.push(IntConstraint::MustBe(
                    right_id,
                    int_type,
                    "literal must match typed operand".to_string(),
                ));
            }
        }
        if left_is_literal && !right_is_literal {
            if let Some(int_type) = self.resolve_expression_int_type(right) {
                self.constraints.push(IntConstraint::MustBe(
                    left_id,
                    int_type,
                    "literal must match typed operand".to_string(),
                ));
            }
        }
    }

    fn resolve_expression_int_type<'ast>(&self, expr: &Expression<'ast>) -> Option<IntType> {
        let ty = self.infer_type_from_expression(expr)?;
        self.extract_nested_int_type(&ty)
    }
}
