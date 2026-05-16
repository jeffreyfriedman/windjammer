impl FloatInference {
    fn collect_expression_constraints_binary<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        left: &'ast Expression<'ast>,
        right: &'ast Expression<'ast>,
        op: crate::parser::ast::operators::BinaryOp,
        return_type: Option<&Type>,
    ) {
        // Binary ops require both operands to have same type
        self.collect_expression_constraints(left, return_type);
        self.collect_expression_constraints(right, return_type);

        // For arithmetic ops (+, -, *, /), operands must match
        use crate::parser::ast::operators::BinaryOp;
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                let left_id = self.get_expr_id(left);
                let right_id = self.get_expr_id(right);

                let binary_id = self.get_expr_id(expr);
                self.constraints.push(Constraint::MustMatch(
                    left_id,
                    right_id,
                    format!("binary operation {:?}", op),
                ));
                self.constraints.push(Constraint::MustMatch(
                    left_id,
                    binary_id,
                    "binary result matches LHS".to_string(),
                ));

                if let Expression::Identifier { name, .. } = left {
                    if let Some(var_type) = self
                        .var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                    {
                        if let Some(float_ty) = self.extract_float_type(var_type) {
                            match float_ty {
                                FloatType::F32 => {
                                    self.constraints.push(Constraint::MustBeF32(
                                        right_id,
                                        format!("binary op RHS: {} has type f32", name),
                                    ));
                                }
                                FloatType::F64 => {
                                    self.constraints.push(Constraint::MustBeF64(
                                        right_id,
                                        format!("binary op RHS: {} has type f64", name),
                                    ));
                                }
                                FloatType::Unknown => {}
                            }
                        }
                    }
                }
                if let Expression::Identifier { name, .. } = right {
                    if let Some(var_type) = self
                        .var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                    {
                        if let Some(float_ty) = self.extract_float_type(var_type) {
                            match float_ty {
                                FloatType::F32 => {
                                    self.constraints.push(Constraint::MustBeF32(
                                        left_id,
                                        format!("binary op LHS: {} has type f32", name),
                                    ));
                                }
                                FloatType::F64 => {
                                    self.constraints.push(Constraint::MustBeF64(
                                        left_id,
                                        format!("binary op LHS: {} has type f64", name),
                                    ));
                                }
                                FloatType::Unknown => {}
                            }
                        }
                    }
                }

                if let Expression::Identifier { name, .. } = right {
                    if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            value_id,
                            format!("backward propagation: {} used with typed LHS", name),
                        ));
                    }
                }
                if let Expression::Identifier { name, .. } = left {
                    if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                        self.constraints.push(Constraint::MustMatch(
                            right_id,
                            value_id,
                            format!("backward propagation: {} used with typed RHS", name),
                        ));
                    }
                }

                self.constrain_nested_floats(left, return_type);
                self.constrain_nested_floats(right, return_type);
            }
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                let left_id = self.get_expr_id(left);
                let right_id = self.get_expr_id(right);
                self.constraints.push(Constraint::MustMatch(
                    left_id,
                    right_id,
                    format!("comparison {:?} operands", op),
                ));
                if let Expression::Identifier { name, .. } = left {
                    if let Some(var_type) = self
                        .var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                    {
                        if let Some(float_ty) = self.extract_float_type(var_type) {
                            match float_ty {
                                FloatType::F32 => {
                                    self.constraints.push(Constraint::MustBeF32(
                                        right_id,
                                        format!("comparison RHS: {} has type f32", name),
                                    ));
                                }
                                FloatType::F64 => {
                                    self.constraints.push(Constraint::MustBeF64(
                                        right_id,
                                        format!("comparison RHS: {} has type f64", name),
                                    ));
                                }
                                FloatType::Unknown => {}
                            }
                        }
                    }
                }
                if let Expression::Identifier { name, .. } = right {
                    if let Some(var_type) = self
                        .var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                    {
                        if let Some(float_ty) = self.extract_float_type(var_type) {
                            match float_ty {
                                FloatType::F32 => {
                                    self.constraints.push(Constraint::MustBeF32(
                                        left_id,
                                        format!("comparison LHS: {} has type f32", name),
                                    ));
                                }
                                FloatType::F64 => {
                                    self.constraints.push(Constraint::MustBeF64(
                                        left_id,
                                        format!("comparison LHS: {} has type f64", name),
                                    ));
                                }
                                FloatType::Unknown => {}
                            }
                        }
                    }
                }
                if let Expression::Identifier { name, .. } = right {
                    if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            value_id,
                            format!("backward propagation: {} in comparison", name),
                        ));
                    }
                }
                if let Expression::Identifier { name, .. } = left {
                    if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                        self.constraints.push(Constraint::MustMatch(
                            right_id,
                            value_id,
                            format!("backward propagation: {} in comparison", name),
                        ));
                    }
                }
            }
            _ => {
                // Logical ops (&&, ||) don't constrain float types
            }
        }
    }
}
