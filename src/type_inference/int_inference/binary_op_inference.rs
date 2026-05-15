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
                self.constraints.push(IntConstraint::MustMatch(
                    left_id,
                    right_id,
                    format!("binary op {:?}", op),
                ));

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

                // TDD FIX: Propagate type from left side (identifier or field access)
                let left_int_ty = match left {
                    Expression::Identifier { name, .. } => self
                        .var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                        .and_then(|ty| self.extract_int_type(ty)),
                    Expression::FieldAccess { object, field, .. } => {
                        // Handle self.field or obj.field
                        if let Expression::Identifier { ref name, .. } = **object {
                            // Get struct name from variable or self
                            let struct_name = if name == "self" {
                                self.current_impl_type.as_ref()
                            } else {
                                self.var_types.get(name.as_str()).and_then(|ty| {
                                    if let Type::Custom(sname) = ty {
                                        Some(sname)
                                    } else {
                                        None
                                    }
                                })
                            };

                            struct_name.and_then(|sname| {
                                self.lookup_struct_fields(sname.as_str())
                                    .and_then(|fields| fields.get(field.as_str()))
                                    .and_then(|field_ty| self.extract_int_type(field_ty))
                            })
                        } else {
                            None
                        }
                    }
                    _ => {
                        // TDD FIX: Fallback to expression type inference for complex expressions
                        self.infer_type_from_expression(left)
                            .and_then(|ty| self.extract_int_type(&ty))
                    }
                };

                if let Some(int_ty) = left_int_ty {
                    self.constraints.push(IntConstraint::MustBe(
                        right_id,
                        int_ty,
                        format!("LHS has type {:?}", int_ty),
                    ));
                }

                // TDD FIX: Propagate type from right side (identifier or field access)
                let right_int_ty = match right {
                    Expression::Identifier { name, .. } => self
                        .var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                        .and_then(|ty| self.extract_int_type(ty)),
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier { ref name, .. } = **object {
                            let struct_name = if name == "self" {
                                self.current_impl_type.as_ref()
                            } else {
                                self.var_types.get(name.as_str()).and_then(|ty| {
                                    if let Type::Custom(sname) = ty {
                                        Some(sname)
                                    } else {
                                        None
                                    }
                                })
                            };

                            struct_name.and_then(|sname| {
                                self.lookup_struct_fields(sname.as_str())
                                    .and_then(|fields| fields.get(field.as_str()))
                                    .and_then(|field_ty| self.extract_int_type(field_ty))
                            })
                        } else {
                            None
                        }
                    }
                    _ => {
                        // TDD FIX: Fallback to expression type inference
                        self.infer_type_from_expression(right)
                            .and_then(|ty| self.extract_int_type(&ty))
                    }
                };

                if let Some(int_ty) = right_int_ty {
                    self.constraints.push(IntConstraint::MustBe(
                        left_id,
                        int_ty,
                        format!("RHS has type {:?}", int_ty),
                    ));
                }
            }
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                self.constraints.push(IntConstraint::MustMatch(
                    left_id,
                    right_id,
                    format!("comparison {:?}", op),
                ));

                // TDD FIX: When comparing with .len() (returns usize), constrain literal to usize
                // e.g., items.len() > 0 → 0 should be usize, not i32
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

                // TDD FIX: Propagate type from left side for comparisons
                // First try pattern matching for direct field access/identifier
                let left_int_ty = match left {
                    Expression::Identifier { name, .. } => self
                        .var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                        .and_then(|ty| self.extract_int_type(ty)),
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier { ref name, .. } = **object {
                            let struct_name = if name == "self" {
                                self.current_impl_type.as_ref()
                            } else {
                                self.var_types.get(name.as_str()).and_then(|ty| {
                                    if let Type::Custom(sname) = ty {
                                        Some(sname)
                                    } else {
                                        None
                                    }
                                })
                            };

                            struct_name.and_then(|sname| {
                                self.lookup_struct_fields(sname.as_str())
                                    .and_then(|fields| fields.get(field.as_str()))
                                    .and_then(|field_ty| self.extract_int_type(field_ty))
                            })
                        } else {
                            None
                        }
                    }
                    _ => {
                        // TDD FIX: Fallback to expression type inference for complex expressions
                        // This handles cases like (self.count % 60) where the result type matters
                        self.infer_type_from_expression(left)
                            .and_then(|ty| self.extract_int_type(&ty))
                    }
                };

                if let Some(int_ty) = left_int_ty {
                    self.constraints.push(IntConstraint::MustBe(
                        right_id,
                        int_ty,
                        format!("comparison LHS has type {:?}", int_ty),
                    ));
                }

                // TDD FIX: Propagate type from right side for comparisons
                let right_int_ty = match right {
                    Expression::Identifier { name, .. } => self
                        .var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                        .and_then(|ty| self.extract_int_type(ty)),
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier { ref name, .. } = **object {
                            let struct_name = if name == "self" {
                                self.current_impl_type.as_ref()
                            } else {
                                self.var_types.get(name.as_str()).and_then(|ty| {
                                    if let Type::Custom(sname) = ty {
                                        Some(sname)
                                    } else {
                                        None
                                    }
                                })
                            };

                            struct_name.and_then(|sname| {
                                self.lookup_struct_fields(sname.as_str())
                                    .and_then(|fields| fields.get(field.as_str()))
                                    .and_then(|field_ty| self.extract_int_type(field_ty))
                            })
                        } else {
                            None
                        }
                    }
                    _ => {
                        // TDD FIX: Fallback to expression type inference
                        self.infer_type_from_expression(right)
                            .and_then(|ty| self.extract_int_type(&ty))
                    }
                };

                if let Some(int_ty) = right_int_ty {
                    self.constraints.push(IntConstraint::MustBe(
                        left_id,
                        int_ty,
                        format!("comparison RHS has type {:?}", int_ty),
                    ));
                }
            }
            _ => {}
        }

        self.collect_expression_constraints(left, return_type);
        self.collect_expression_constraints(right, return_type);
    }
}
