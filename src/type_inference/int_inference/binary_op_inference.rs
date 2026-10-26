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
                    let left_returns_usize = self.method_call_returns_usize(left);
                    let right_returns_usize = self.method_call_returns_usize(right);
                    if left_returns_usize && right_is_literal {
                        self.constraints.push(IntConstraint::MustBe(
                            right_id,
                            IntType::Usize,
                            "arithmetic with usize-returning method".to_string(),
                        ));
                    }
                    if right_returns_usize && left_is_literal {
                        self.constraints.push(IntConstraint::MustBe(
                            left_id,
                            IntType::Usize,
                            "arithmetic with usize-returning method".to_string(),
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
                let left_returns_usize = self.method_call_returns_usize(left);
                let right_returns_usize = self.method_call_returns_usize(right);
                if left_returns_usize && right_is_literal {
                    self.constraints.push(IntConstraint::MustBe(
                        right_id,
                        IntType::Usize,
                        "comparison with usize-returning method".to_string(),
                    ));
                }
                if right_returns_usize && left_is_literal {
                    self.constraints.push(IntConstraint::MustBe(
                        left_id,
                        IntType::Usize,
                        "comparison with usize-returning method".to_string(),
                    ));
                }

                self.propagate_typed_operand_to_literal(left, right, left_id, right_id, left_is_literal, right_is_literal, "comparison");
                // f32 slab compares (`tmax >= tmin`, `tmax >= 0.0`) are not
                // integer-width constraints. MustMatch here reported Isize vs I64 and
                // aborted engine library transpile.
                if !self.expression_is_float_compare_operand(left)
                    && !self.expression_is_float_compare_operand(right)
                {
                    self.constraints.push(IntConstraint::MustMatch(
                        left_id,
                        right_id,
                        "comparison operands must share integer width".to_string(),
                    ));
                }
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
        // A "literal-only" operand (`7`, `(1 + 7)`, `8 * 5`) has no type of its own: every literal in
        // it takes the typed peer's type (one class: `((1 + 7) - pu) + 7` with `pu: u32` is all u32).
        for (lit_side, typed_side) in [(right, left), (left, right)] {
            let mut leaves = Vec::new();
            if !Self::literal_only_leaves(lit_side, &mut leaves) || leaves.is_empty() {
                continue;
            }
            if Self::literal_only_leaves(typed_side, &mut Vec::new()) {
                continue;
            }
            if let Some(int_type) = self.resolve_expression_int_type(typed_side) {
                // Codegen folds `1 + 7` to one literal that keeps the Binary's location: type the root too.
                let root_id = self.get_expr_id(lit_side);
                self.constraints.push(IntConstraint::MustBe(root_id, int_type, "literal-only operand follows typed peer".to_string()));
                for leaf in leaves {
                    let id = self.get_expr_id(leaf);
                    self.constraints.push(IntConstraint::MustBe(
                        id,
                        int_type,
                        "literal must match typed operand".to_string(),
                    ));
                }
            }
        }
        let _ = (left_id, right_id, left_is_literal, right_is_literal);
    }

    /// True when `expr` is built only from unsuffixed integer literals and arithmetic; collects the literals.
    fn literal_only_leaves<'a, 'ast>(expr: &'a Expression<'ast>, out: &mut Vec<&'a Expression<'ast>>) -> bool {
        use crate::parser::ast::operators::{BinaryOp, UnaryOp};
        match expr {
            Expression::Literal { value: crate::parser::Literal::Int(_), .. } => {
                out.push(expr);
                true
            }
            Expression::Unary { op: UnaryOp::Neg, operand, .. } => Self::literal_only_leaves(operand, out),
            Expression::Binary { left, op, right, .. }
                if matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                ) =>
            {
                Self::literal_only_leaves(left, out) && Self::literal_only_leaves(right, out)
            }
            _ => false,
        }
    }

    /// Integer type of `expr` when it is knowable without constraint solving: a typed leaf, a cast,
    /// or arithmetic whose operands are typed (`2 * seg` is `seg`'s type). An unsuffixed literal next
    /// to such an expression takes its type; the `int` default applies only to unconstrained classes.
    fn resolve_expression_int_type<'ast>(&self, expr: &Expression<'ast>) -> Option<IntType> {
        use crate::parser::ast::operators::{BinaryOp, UnaryOp};
        // A literal-only expression has no type of its own to offer a peer.
        if Self::literal_only_leaves(expr, &mut Vec::new()) {
            return None;
        }
        match expr {
            Expression::Binary { left, op, right, .. }
                if matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Mod
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                ) =>
            {
                self.resolve_expression_int_type(left)
                    .or_else(|| self.resolve_expression_int_type(right))
            }
            // Shifts take the left operand's type; the shift amount does not constrain it.
            Expression::Binary { left, op: BinaryOp::Shl | BinaryOp::Shr, .. } => {
                self.resolve_expression_int_type(left)
            }
            Expression::Unary { op: UnaryOp::Neg, operand, .. } => self.resolve_expression_int_type(operand),
            Expression::Cast { type_, .. } => self.extract_int_type(type_),
            _ => {
                let ty = self.infer_type_from_expression(expr)?;
                self.extract_nested_int_type(&ty)
            }
        }
    }

    // float compares must not enter integer MustMatch.
    fn expression_is_float_compare_operand<'ast>(&self, expr: &Expression<'ast>) -> bool {
        use crate::parser::Literal;
        if matches!(
            expr,
            Expression::Literal {
                value: Literal::Float(_),
                ..
            }
        ) {
            return true;
        }
        self.infer_type_from_expression(expr).is_some_and(|t| {
            matches!(t, Type::Float)
                || matches!(t, Type::Custom(n) if n == "f32" || n == "f64")
        })
    }
}
