// Integer-width constraints for unary operators and closure bodies.

impl IntInference {
    pub(crate) fn collect_unary_or_closure_constraints<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        return_type: Option<&Type>,
    ) {
        match expr {
            Expression::Unary { op, operand, .. } => {
                self.collect_unary_constraints(expr, format!("{op:?}"), operand, return_type)
            }
            Expression::Closure { parameters, body, .. } => self.collect_closure_constraints(parameters, body, &[]),
            _ => {}
        }
    }

    pub(crate) fn collect_unary_constraints<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        op: String,
        operand: &Expression<'ast>,
        return_type: Option<&Type>,
    ) {
                // `&v`, `&mut v`, `*r`, `-x`, `!x` have the width of their operand (references are
                // transparent to integer width), so a call `f(&v)` with `f(v: &Vec<i64>)` reaches `v`.
                self.collect_expression_constraints(operand, return_type);
                let operand_id = self.get_expr_id(operand);
                let expr_id = self.get_expr_id(expr);
                if expr_id != operand_id {
                    self.constraints.push(IntConstraint::MustMatch(
                        expr_id,
                        operand_id,
                        format!("unary {op} keeps its operand's integer width"),
                    ));
                }

    }

    pub(crate) fn collect_closure_constraints<'ast>(
        &mut self,
        parameters: &[String],
        body: &Expression<'ast>,
        // Immediately-invoked closure (`(|v| v * 2)(x)`, the lowering of `x |> (|v| v * 2)`): an untyped
        // parameter is bound to its argument, so the body's literals take the argument's width.
        args: &[ExprId],
    ) {
                // A closure body is ordinary code: its literals follow the captured locals they meet
                // (`|| base * 2` with `base: int`). Parameters shadow outer names for the duration.
                let mut saved: Vec<(String, Option<ExprId>, Option<Type>)> = Vec::new();
                for (i, p) in parameters.iter().enumerate() {
                    let (name, ty) = match p.split_once(':') {
                        Some((n, t)) => (n.trim().to_string(), Some(self.parse_type_from_string(t.trim()))),
                        None => (p.trim().to_string(), None),
                    };
                    saved.push((
                        name.clone(),
                        self.var_assignments.remove(&name),
                        match ty {
                            Some(t) => self.var_types.insert(name.clone(), t),
                            None => self.var_types.remove(&name),
                        },
                    ));
                    if let Some(&arg) = args.get(i) {
                        self.var_assignments.insert(name, arg);
                    }
                }
                self.collect_expression_constraints(body, None);
                for (name, assignment, ty) in saved {
                    match assignment {
                        Some(a) => self.var_assignments.insert(name.clone(), a),
                        None => self.var_assignments.remove(&name),
                    };
                    match ty {
                        Some(t) => self.var_types.insert(name, t),
                        None => self.var_types.remove(&name),
                    };
                }

    }
}
