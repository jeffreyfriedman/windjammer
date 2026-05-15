impl FloatInference {
    /// Collect constraints from an expression
    fn collect_expression_constraints<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        return_type: Option<&Type>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                // TDD FIX: Constrain identifier to its declared type (function param, let with type annotation, const)
                let var_type = self
                    .var_types
                    .get(name)
                    .or_else(|| self.const_types.get(name));
                if let Some(var_type) = var_type {
                    if let Some(float_ty) = self.extract_float_type(var_type) {
                        let id = self.get_expr_id(expr);
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    id,
                                    format!("identifier {} has type f32", name),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    id,
                                    format!("identifier {} has type f64", name),
                                ));
                            }
                            FloatType::Unknown => {}
                        }
                    }
                }

                // TDD FIX: Variable assignment type propagation
                // When a variable is used (e.g., "det" in "1.0 / det"),
                // link it to its assigned value so type propagates through
                // Example: let det = a * b; let inv_det = 1.0 / det
                //   -> det must match (a * b) -> 1.0 must match det -> 1.0 matches a's type
                if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                    let identifier_id = self.get_expr_id(expr);
                    self.constraints.push(Constraint::MustMatch(
                        identifier_id,
                        value_id,
                        format!("variable {} matches its assigned value", name),
                    ));
                }
            }
            Expression::Binary { left, right, op, .. } => {
                self.collect_expression_constraints_binary(expr, left, right, *op, return_type);
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                self.collect_expression_constraints_method_call(
                    expr,
                    object,
                    method.as_str(),
                    arguments,
                    return_type,
                );
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.collect_expression_constraints_call(expr, function, arguments, return_type);
            }
            Expression::StructLiteral { name, fields, .. } => {
                // THE WINDJAMMER WAY: Constrain struct field expressions to match field types

                // Collect field type constraints (two-phase to avoid borrow checker issues)
                let field_constraints: Vec<(String, &'ast Expression<'ast>, FloatType)> =
                    if let Some(struct_fields) = self.lookup_struct_fields(name) {
                        fields
                            .iter()
                            .filter_map(|(field_name, field_expr)| {
                                if let Some(field_type) = struct_fields.get(field_name) {
                                    self.extract_float_type(field_type)
                                        .map(|float_ty| (field_name.clone(), *field_expr, float_ty))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

                // Now create constraints with mutable access
                for (field_name, field_expr, float_ty) in field_constraints {
                    let expr_id = self.get_expr_id(field_expr);
                    let constraint = match float_ty {
                        FloatType::F32 => Constraint::MustBeF32(
                            expr_id,
                            format!("struct {}.{} is f32", name, field_name),
                        ),
                        FloatType::F64 => Constraint::MustBeF64(
                            expr_id,
                            format!("struct {}.{} is f64", name, field_name),
                        ),
                        FloatType::Unknown => continue,
                    };
                    self.constraints.push(constraint);
                }

                // Recursively collect constraints from field expressions
                for (_field_name, expr) in fields {
                    self.collect_expression_constraints(expr, return_type);
                }
            }
            Expression::Tuple { elements, .. } => {
                // Tuple expression: match elements with return type positions
                if let Some(Type::Tuple(tuple_types)) = return_type {
                    for (i, elem) in elements.iter().enumerate() {
                        if let Some(elem_type) = tuple_types.get(i) {
                            // Recurse with the specific type for this position
                            self.collect_expression_constraints(elem, Some(elem_type));

                            // If this position is a float type, constrain the element
                            if let Some(float_ty) = self.extract_float_type(elem_type) {
                                let elem_id = self.get_expr_id(elem);
                                let constraint = match float_ty {
                                    FloatType::F32 => Constraint::MustBeF32(
                                        elem_id,
                                        format!("tuple element {}", i),
                                    ),
                                    FloatType::F64 => Constraint::MustBeF64(
                                        elem_id,
                                        format!("tuple element {}", i),
                                    ),
                                    FloatType::Unknown => continue,
                                };
                                self.constraints.push(constraint);

                                // If element is an identifier, also constrain its assigned value
                                if let Expression::Identifier { name, .. } = elem {
                                    if let Some(&value_id) = self.var_assignments.get(name.as_str())
                                    {
                                        let value_constraint = match float_ty {
                                            FloatType::F32 => Constraint::MustBeF32(
                                                value_id,
                                                format!("variable {} assigned value", name),
                                            ),
                                            FloatType::F64 => Constraint::MustBeF64(
                                                value_id,
                                                format!("variable {} assigned value", name),
                                            ),
                                            FloatType::Unknown => continue,
                                        };
                                        self.constraints.push(value_constraint);
                                    }
                                }
                            }
                        } else {
                            // No type info for this position, recurse without constraint
                            self.collect_expression_constraints(elem, None);
                        }
                    }
                } else {
                    // No tuple type info, just recurse
                    for elem in elements {
                        self.collect_expression_constraints(elem, None);
                    }
                }
            }
            Expression::Cast {
                expr: inner, type_, ..
            } => {
                // Cast expression provides explicit type constraint
                self.collect_expression_constraints(inner, return_type);

                if let Some(float_ty) = self.extract_float_type(type_) {
                    let cast_id = self.get_expr_id(expr);
                    let cast_constraint = match float_ty {
                        FloatType::F32 => {
                            Constraint::MustBeF32(cast_id, "cast expression result f32".to_string())
                        }
                        FloatType::F64 => {
                            Constraint::MustBeF64(cast_id, "cast expression result f64".to_string())
                        }
                        FloatType::Unknown => return,
                    };
                    self.constraints.push(cast_constraint);

                    let inner_id = self.get_expr_id(inner);
                    let constraint = match float_ty {
                        FloatType::F32 => {
                            Constraint::MustBeF32(inner_id, "cast to f32".to_string())
                        }
                        FloatType::F64 => {
                            Constraint::MustBeF64(inner_id, "cast to f64".to_string())
                        }
                        FloatType::Unknown => return,
                    };
                    self.constraints.push(constraint);
                }
            }
            Expression::Block { statements, .. } => {
                // TDD FIX: Traverse block expressions (e.g., let x = { match ... })
                // Match statements inside blocks weren't being visited!
                for stmt in statements {
                    self.collect_statement_constraints(stmt, return_type);
                }
                // TDD FIX: Constrain block result to return_type when block is used as value
                // For `x = if c { a } else { b }`, the block wraps the If; constrain both branches
                if let Some(last_stmt) = statements.last() {
                    let block_id = self.get_expr_id(expr);
                    if let Statement::If {
                        then_block,
                        else_block,
                        ..
                    } = last_stmt
                    {
                        if let (Some(then_last), Some(else_stmts)) = (then_block.last(), else_block)
                        {
                            if let Some(else_last) = else_stmts.last() {
                                if let (
                                    Statement::Expression { expr: te, .. },
                                    Statement::Expression { expr: ee, .. },
                                ) = (then_last, else_last)
                                {
                                    let then_id = self.get_expr_id(te);
                                    let else_id = self.get_expr_id(ee);
                                    self.constraints.push(Constraint::MustMatch(
                                        block_id,
                                        then_id,
                                        "block result from if then".to_string(),
                                    ));
                                    self.constraints.push(Constraint::MustMatch(
                                        block_id,
                                        else_id,
                                        "block result from if else".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                // TDD FIX: Constrain field access to field's type
                // e.g., self.vy (f32) or self.cell_size (f32) should constrain the FieldAccess expression to f32

                // First, get the ID for this FieldAccess expression (before mutating self)
                let field_access_id = self.get_expr_id(expr);

                // Recurse into object
                self.collect_expression_constraints(object, return_type);

                // Determine the struct type that contains this field
                let struct_name: Option<String> =
                    if let Expression::Identifier { name, .. } = object {
                        if name == "self" {
                            // Case 1: self.field - use current impl type context
                            self.current_impl_type.clone()
                        } else {
                            // Case 2: variable.field - look up variable's type
                            self.var_types
                                .get(name)
                                .and_then(|var_type| match var_type {
                                    Type::Custom(name) => Some(name.clone()),
                                    _ => None,
                                })
                        }
                    } else {
                        // TDD FIX: Chained FieldAccess - self.player.position.x
                        // Use infer_type_from_expression to resolve object's type
                        self.infer_type_from_expression(object)
                            .and_then(|obj_ty| match &obj_ty {
                                Type::Custom(name) => Some(name.clone()),
                                _ => None,
                            })
                    };

                // If we know the struct type, constrain the FieldAccess expression to the field's type
                if let Some(struct_name) = struct_name {
                    let field_map = if matches!(
                        *object,
                        Expression::Identifier { ref name, .. } if name == "self"
                    ) {
                        self.current_impl_type
                            .as_deref()
                            .and_then(|ty| self.lookup_struct_fields_for_impl_type(ty))
                    } else {
                        self.lookup_struct_fields(&struct_name)
                    };

                    if let Some(field_types) = field_map {
                        if let Some(field_type) = field_types.get(field) {
                            if let Some(float_ty) = self.extract_float_type(field_type) {
                                // TDD FIX: Constrain the FieldAccess expression itself, not the object!
                                // This is critical for binary ops: `self.vy * 0.5` needs the entire
                                // FieldAccess expression to be constrained, which then propagates
                                // through MustMatch to the literal
                                match float_ty {
                                    FloatType::F32 => {
                                        self.constraints.push(Constraint::MustBeF32(
                                            field_access_id,
                                            format!("{}.{} is f32", struct_name, field),
                                        ));
                                    }
                                    FloatType::F64 => {
                                        self.constraints.push(Constraint::MustBeF64(
                                            field_access_id,
                                            format!("{}.{} is f64", struct_name, field),
                                        ));
                                    }
                                    FloatType::Unknown => {}
                                }
                            }
                        }
                    }
                }
            }
            Expression::Index { object, index, .. } => {
                // TDD FIX: Index expression (arr[i]) yields element type - constrain for binary ops
                // e.g., arr[i] / 2.0 when arr: Vec<f32> → Index is f32, so 2.0 must be f32
                self.collect_expression_constraints(object, return_type);
                self.collect_expression_constraints(index, return_type);

                if let Some(elem_type) = self
                    .infer_type_from_expression(object)
                    .and_then(|ty| self.extract_vec_element_type(&ty))
                {
                    if let Some(float_ty) = self.extract_float_type(&elem_type) {
                        let index_expr_id = self.get_expr_id(expr);
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    index_expr_id,
                                    "Index yields Vec<f32> element".to_string(),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    index_expr_id,
                                    "Index yields Vec<f64> element".to_string(),
                                ));
                            }
                            FloatType::Unknown => {}
                        }
                    }
                }
            }
            Expression::Unary { op, operand, .. } => {
                use crate::parser::ast::operators::UnaryOp;
                self.collect_expression_constraints(operand, return_type);
                if matches!(op, UnaryOp::Deref) {
                    let unary_id = self.get_expr_id(expr);
                    let operand_id = self.get_expr_id(operand);
                    self.constraints.push(Constraint::MustMatch(
                        unary_id,
                        operand_id,
                        "dereference has same float type as pointee".to_string(),
                    ));
                }
            }
            Expression::Literal { value, .. } => {
                // TDD FIX: Constrain float literals to return type
                if matches!(value, crate::parser::ast::literals::Literal::Float(_)) {
                    if let Some(float_ty) = return_type.and_then(|rt| self.extract_float_type(rt)) {
                        let expr_id = self.get_expr_id(expr);
                        let constraint = match float_ty {
                            FloatType::F32 => Constraint::MustBeF32(
                                expr_id,
                                "literal constrained by return type".to_string(),
                            ),
                            FloatType::F64 => Constraint::MustBeF64(
                                expr_id,
                                "literal constrained by return type".to_string(),
                            ),
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Expression::MacroInvocation { name, args, .. } => {
                // TDD FIX: assert_eq!/assert_ne! - both args must have same type
                // assert_eq!(transform.x, 15.0) → 15.0 should infer f32 from transform.x
                if (name == "assert_eq" || name == "assert_ne") && args.len() >= 2 {
                    // Recurse into args first (FieldAccess adds MustBeF32 for transform.x)
                    for arg in args {
                        self.collect_expression_constraints(arg, return_type);
                    }
                    // Link first and second arg: same type required
                    let first_id = self.get_expr_id(args[0]);
                    let second_id = self.get_expr_id(args[1]);
                    self.constraints.push(Constraint::MustMatch(
                        first_id,
                        second_id,
                        format!("{}! requires both arguments to have same type", name),
                    ));
                    return; // Already recursed
                }
                // Other macros (format!, vec!, etc.): just recurse into args
                for arg in args {
                    self.collect_expression_constraints(arg, return_type);
                }
            }
            _ => {}
        }
    }

}
