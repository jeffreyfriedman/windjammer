impl IntInference {
    fn constrain_call_result_int_type<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        return_type: Option<&Type>,
        reason: &str,
    ) {
        if let Some(int_ty) = return_type.and_then(|t| self.extract_int_type(t)) {
            let id = self.get_expr_id(expr);
            self.constraints.push(IntConstraint::MustBe(
                id,
                int_ty,
                reason.to_string(),
            ));
        }
    }

    fn collect_expression_constraints<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        return_type: Option<&Type>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                let id = self.get_expr_id(expr);
                
                // TDD FIX REMOVED: Don't link identifier uses to assignments.
                // This caused backward propagation where:
                //   let n = data.len() as i32  // n is i32
                //   data[n]  // Index forces usize
                // would make n usize at declaration, breaking comparisons like `if idx >= n`
                //
                // Instead, we'll insert casts at use sites during code generation:
                //   data[n as usize]  // Cast happens here, not at declaration
                
                if let Some(var_type) = self
                    .var_types
                    .get(name)
                    .or_else(|| self.const_types.get(name))
                {
                    if let Some(int_ty) = self.extract_int_type(var_type) {
                        self.constraints.push(IntConstraint::MustBe(
                            id,
                            int_ty,
                            format!("identifier {} type", name),
                        ));
                    }
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.collect_expression_constraints(function, return_type);

                let func_name = match function {
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    Expression::FieldAccess { object, field, .. } => {
                        // Resolve the object's type name for qualified method lookup.
                        // Handles: identifier.method(), self.field.method(), expr.method()
                        let resolved_type_name =
                            if let Expression::Identifier { name: obj_name, .. } = object {
                                if obj_name.chars().next().is_some_and(|c| c.is_lowercase()) {
                                    if obj_name == "self" {
                                        self.current_impl_type.clone()
                                    } else {
                                        self.var_types.get(obj_name).and_then(|ty| match ty {
                                            Type::Custom(n) => Some(n.clone()),
                                            _ => None,
                                        })
                                    }
                                } else {
                                    Some(obj_name.clone())
                                }
                            } else {
                                // Nested expression (e.g., self.entries.remove) - resolve via type inference
                                self.infer_type_from_expression(object)
                                    .and_then(|ty| match &ty {
                                        Type::Custom(n) => Some(n.clone()),
                                        Type::Parameterized(n, _) => Some(n.clone()),
                                        Type::Vec(_) => Some("Vec".to_string()),
                                        _ => None,
                                    })
                            };
                        resolved_type_name
                            .as_ref()
                            .map(|type_name| format!("{}::{}", type_name, field))
                    }
                    _ => None,
                };

                // TDD FIX: assert_eq/assert_ne - both args must have same int type
                if (func_name.as_deref() == Some("assert_eq")
                    || func_name.as_deref() == Some("assert_ne"))
                    && arguments.len() >= 2
                {
                    for (_label, arg) in arguments {
                        self.collect_expression_constraints(arg, return_type);
                    }
                    if let (Some((_, first)), Some((_, second))) =
                        (arguments.first(), arguments.get(1))
                    {
                        let first_id = self.get_expr_id(first);
                        let second_id = self.get_expr_id(second);
                        self.constraints.push(IntConstraint::MustMatch(
                            first_id,
                            second_id,
                            "assert_eq/assert_ne requires both arguments to have same type"
                                .to_string(),
                        ));
                    }
                    return;
                }

                // TDD FIX: Some(expr) with expected Option<T> - constrain argument to T
                // Handles: Node { children: Some(vec![2, 3]) } where children: Option<Vec<int>>
                let arg_expected_type: Option<Type> = if arguments.len() == 1 {
                    if func_name.as_deref() == Some("Some")
                        || func_name.as_deref() == Some("Option::Some")
                    {
                        return_type
                            .and_then(|ty| self.extract_option_inner_type(ty))
                            .or_else(|| {
                                func_name
                                    .as_ref()
                                    .and_then(|name| self.function_signatures.get(name))
                                    .and_then(|(params, _)| params.first())
                                    .and_then(|param_ty| self.extract_option_inner_type(param_ty))
                            })
                    } else {
                        None
                    }
                } else {
                    None
                };

                let func_sig = func_name
                    .as_ref()
                    .and_then(|name| self.function_signatures.get(name).cloned());

                if let Some((param_types, ret)) = func_sig {
                    let param_offset = if param_types.len() == arguments.len() + 1 {
                        1
                    } else {
                        0
                    };
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        let arg_ty: Option<&Type> = if i == 0 {
                            arg_expected_type
                                .as_ref()
                                .or_else(|| param_types.get(i + param_offset))
                        } else {
                            param_types.get(i + param_offset)
                        };
                        self.collect_expression_constraints(arg, arg_ty.or(return_type));
                        if let Some(param_type) = param_types.get(i + param_offset) {
                            if let Some(int_ty) = self.extract_nested_int_type(param_type) {
                                let arg_id = self.get_expr_id(arg);
                                self.constraints.push(IntConstraint::MustBe(
                                    arg_id,
                                    int_ty,
                                    format!("parameter {} of function", i),
                                ));
                            }
                        }
                    }
                    self.constrain_call_result_int_type(
                        expr,
                        ret.as_ref(),
                        "function call return",
                    );
                } else {
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        let arg_ty: Option<&Type> = if i == 0 {
                            arg_expected_type.as_ref()
                        } else {
                            None
                        };
                        self.collect_expression_constraints(arg, arg_ty.or(return_type));
                        // TDD FIX: Add MustBe for argument when we have int type (e.g. Some(42), Option<Option<int>>)
                        if i == 0 {
                            if let Some(ref at) = arg_expected_type {
                                if let Some(int_ty) = self.extract_nested_int_type(at) {
                                    let arg_id = self.get_expr_id(arg);
                                    self.constraints.push(IntConstraint::MustBe(
                                        arg_id,
                                        int_ty,
                                        "Option::Some argument type".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                self.collect_expression_constraints(object, return_type);

                if let Some((param_types, ret)) =
                    self.resolve_method_signature(object, method, arguments.len(), return_type)
                {
                    self.apply_method_param_int_constraints(
                        &param_types,
                        arguments,
                        method,
                        return_type,
                    );
                    self.constrain_call_result_int_type(
                        expr,
                        ret.as_ref(),
                        &format!("{method}() return"),
                    );
                } else {
                    for (_label, arg) in arguments {
                        self.collect_expression_constraints(arg, return_type);
                    }
                }
            }
            Expression::StructLiteral { name, fields, .. } => {
                // Two-phase to avoid borrow checker: collect field types first, then iterate
                let field_data: Vec<(&'ast Expression<'ast>, IntType, String, Option<Type>)> = self
                    .lookup_struct_fields(name)
                    .map_or_else(Vec::new, |struct_fields| {
                        fields
                            .iter()
                            .map(|(field_name, field_expr)| {
                                let field_type = struct_fields.get(field_name).cloned();
                                let (int_ty, reason) = field_type
                                    .as_ref()
                                    .and_then(|ft| {
                                        self.extract_nested_int_type(ft).map(|it| {
                                            (it, format!("struct {}.{}", name, field_name))
                                        })
                                    })
                                    .unwrap_or((IntType::Unknown, String::new()));
                                (*field_expr, int_ty, reason, field_type)
                            })
                            .collect()
                    });
                for (field_expr, int_ty, reason, _) in &field_data {
                    if *int_ty != IntType::Unknown {
                        let expr_id = self.get_expr_id(field_expr);
                        self.constraints.push(IntConstraint::MustBe(
                            expr_id,
                            *int_ty,
                            reason.clone(),
                        ));
                    }
                }
                // TDD FIX: Pass field type when recursing so nested generics (Option<Vec<int>>) propagate
                for (i, (_field_name, expr)) in fields.iter().enumerate() {
                    let expected_type = field_data.get(i).and_then(|(_, _, _, ft)| ft.as_ref());
                    self.collect_expression_constraints(expr, expected_type.or(return_type));
                }
            }
            Expression::Binary {
                left, op, right, ..
            } => {
                self.collect_binary_op_constraints(left, *op, right, return_type);
            }
            Expression::Cast {
                expr: inner, type_, ..
            } => {
                // Cast converts between types - do NOT constrain operand to match target.
                // e.g. (x as usize) is valid when x is i32, u32, etc.
                self.collect_expression_constraints(inner, return_type);
                // Constrain the cast RESULT to the target type (fixes return type conflicts)
                if let Some(int_ty) = self.extract_int_type(type_) {
                    let cast_id = self.get_expr_id(expr);
                    self.constraints.push(IntConstraint::MustBe(
                        cast_id,
                        int_ty,
                        "cast target type".to_string(),
                    ));
                }
            }
            Expression::Tuple { elements, .. } => {
                if let Some(Type::Tuple(tuple_types)) = return_type {
                    for (i, elem) in elements.iter().enumerate() {
                        if let Some(elem_type) = tuple_types.get(i) {
                            self.collect_expression_constraints(elem, Some(elem_type));
                            if let Some(int_ty) = self.extract_int_type(elem_type) {
                                let elem_id = self.get_expr_id(elem);
                                self.constraints.push(IntConstraint::MustBe(
                                    elem_id,
                                    int_ty,
                                    format!("tuple element {}", i),
                                ));
                            }
                        } else {
                            self.collect_expression_constraints(elem, None);
                        }
                    }
                } else {
                    for elem in elements {
                        self.collect_expression_constraints(elem, None);
                    }
                }
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_statement_constraints(stmt, return_type);
                }
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_expression_constraints(object, return_type);
            }
            Expression::Index { object, index, .. } => {
                // TDD FIX: Vec<T>[idx] should infer result as T, not usize
                // Extract Vec element type and constrain Index result accordingly
                if let Some(object_type) = self.infer_type_from_expression(object) {
                    if let Some(elem_type) = self.extract_vec_element_type(&object_type) {
                        if let Some(int_ty) = self.extract_nested_int_type(&elem_type) {
                            let expr_id = self.get_expr_id(expr);
                            self.constraints.push(IntConstraint::MustBe(
                                expr_id,
                                int_ty,
                                format!("Vec<{:?}> element type", int_ty),
                            ));
                        }
                    }
                }

                // Don't pass return_type to object - Vec has its own type
                self.collect_expression_constraints(object, None);

                // TDD FIX: Array indices must be usize in Rust
                let index_id = self.get_expr_id(index);
                self.constraints.push(IntConstraint::MustBe(
                    index_id,
                    IntType::Usize,
                    "array index must be usize".to_string(),
                ));
                self.collect_expression_constraints(index, None);
            }
            Expression::Array { elements, .. } => {
                // TDD FIX: [a, b, c] with expected Vec<T> - constrain elements to T
                if let Some(elem_type) =
                    return_type.and_then(|ty| self.extract_vec_element_type(ty))
                {
                    for elem in elements {
                        self.collect_expression_constraints(elem, Some(&elem_type));
                        if let Some(int_ty) = self.extract_nested_int_type(&elem_type) {
                            let elem_id = self.get_expr_id(elem);
                            self.constraints.push(IntConstraint::MustBe(
                                elem_id,
                                int_ty,
                                "array/vec element type".to_string(),
                            ));
                        }
                    }
                } else {
                    for elem in elements {
                        self.collect_expression_constraints(elem, return_type);
                    }
                }
            }
            Expression::MacroInvocation {
                name,
                args,
                is_repeat,
                ..
            } => {
                // TDD FIX: assert_eq!/assert_ne! - both args must have same int type
                if (*name == "assert_eq" || *name == "assert_ne") && args.len() >= 2 {
                    for arg in args {
                        self.collect_expression_constraints(arg, return_type);
                    }
                    let first_id = self.get_expr_id(args[0]);
                    let second_id = self.get_expr_id(args[1]);
                    self.constraints.push(IntConstraint::MustMatch(
                        first_id,
                        second_id,
                        format!("{}! requires both arguments to have same type", name),
                    ));
                    return;
                }
                // TDD FIX: vec![a, b, c] with expected Vec<T> - constrain elements to T
                if *name == "vec" && !*is_repeat {
                    if let Some(elem_type) =
                        return_type.and_then(|ty| self.extract_vec_element_type(ty))
                    {
                        for arg in args {
                            self.collect_expression_constraints(arg, Some(&elem_type));
                            if let Some(int_ty) = self.extract_nested_int_type(&elem_type) {
                                let arg_id = self.get_expr_id(arg);
                                self.constraints.push(IntConstraint::MustBe(
                                    arg_id,
                                    int_ty,
                                    "vec! element type".to_string(),
                                ));
                            }
                        }
                    } else {
                        for arg in args {
                            self.collect_expression_constraints(arg, return_type);
                        }
                    }
                } else {
                    for arg in args {
                        self.collect_expression_constraints(arg, return_type);
                    }
                }
            }
            Expression::Range { start, end, .. } => {
                // TDD FIX: Ranges like 0..vec.len() should have unified types
                // If end is .len() (returns usize), constrain start to usize
                // Otherwise, unify both expressions to the same type

                // Collect constraints from both expressions
                self.collect_expression_constraints(start, return_type);
                self.collect_expression_constraints(end, return_type);

                // Check if end returns usize (e.g. .len()) — constrain start to match
                let end_returns_usize = self.method_call_returns_usize(end);

                if end_returns_usize {
                    // Constrain start to usize to match .len()
                    let start_id = self.get_expr_id(start);
                    self.constraints.push(IntConstraint::MustBe(
                        start_id,
                        IntType::Usize,
                        "range start must match .len() return type (usize)".to_string(),
                    ));
                } else {
                    // General case: unify both sides of range
                    let start_id = self.get_expr_id(start);
                    let end_id = self.get_expr_id(end);
                    self.constraints.push(IntConstraint::MustMatch(
                        start_id,
                        end_id,
                        "range bounds must have same type".to_string(),
                    ));
                }
            }
            _ => {}
        }
    }
}
