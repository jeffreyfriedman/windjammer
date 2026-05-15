impl FloatInference {
    fn collect_item_constraints<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, .. } => {
                // THE WINDJAMMER WAY: Clear cache for each function to ensure proper scoping
                self.expr_id_cache.clear();

                // TDD FIX: Register function parameters for type tracking
                for param in &decl.parameters {
                    self.var_types
                        .insert(param.name.clone(), param.type_.clone());
                }

                // TDD FIX: Handle implicit returns FIRST (before collecting constraints)
                // This populates var_element_types so that .push()/.insert() can use them
                if let Some(Statement::Expression { expr, .. }) = decl.body.last() {
                    // This is an implicit return - store variable element types
                    if let Some(return_type) = &decl.return_type {
                        self.constrain_expr_to_type(expr, return_type);
                    }
                }

                // Collect return type constraints (now var_element_types is populated)
                for stmt in decl.body.iter() {
                    self.collect_statement_constraints(stmt, decl.return_type.as_ref());
                }

                // Clear parameter types after function (for proper scoping)
                for param in &decl.parameters {
                    self.var_types.remove(&param.name);
                }
            }
            Item::Impl { block, .. } => {
                // TDD FIX: Set current impl type context for `self` field access resolution
                self.current_impl_type = Some(block.type_name.clone());

                // Process methods in impl block
                for func in &block.functions {
                    // THE WINDJAMMER WAY: Clear cache for each method to ensure proper scoping
                    self.expr_id_cache.clear();

                    // TDD FIX: Register function parameters for type tracking
                    for param in &func.parameters {
                        self.var_types
                            .insert(param.name.clone(), param.type_.clone());
                    }

                    // TDD FIX: Handle implicit returns FIRST (before collecting constraints)
                    if let Some(Statement::Expression { expr, .. }) = func.body.last() {
                        if let Some(return_type) = &func.return_type {
                            self.constrain_expr_to_type(expr, return_type);
                        }
                    }

                    for stmt in func.body.iter() {
                        self.collect_statement_constraints(stmt, func.return_type.as_ref());
                    }

                    // Clear parameter types after method (for proper scoping)
                    for param in &func.parameters {
                        self.var_types.remove(&param.name);
                    }
                }

                // Clear impl type context
                self.current_impl_type = None;
            }
            Item::Struct { .. } => {}
            Item::Enum { .. } => {}
            Item::Trait { .. } => {}
            Item::Const {
                name, type_, value, ..
            } => {
                self.collect_expression_constraints(value, None);
                if let Some(float_ty) = self.extract_float_type(type_) {
                    let expr_id = self.get_expr_id(value);
                    match float_ty {
                        FloatType::F32 => {
                            self.constraints.push(Constraint::MustBeF32(
                                expr_id,
                                format!("const {} is f32", name),
                            ));
                        }
                        FloatType::F64 => {
                            self.constraints.push(Constraint::MustBeF64(
                                expr_id,
                                format!("const {} is f64", name),
                            ));
                        }
                        FloatType::Unknown => {}
                    }
                }
            }
            Item::Static {
                name, type_, value, ..
            } => {
                self.collect_expression_constraints(value, None);
                if let Some(float_ty) = self.extract_float_type(type_) {
                    let expr_id = self.get_expr_id(value);
                    match float_ty {
                        FloatType::F32 => {
                            self.constraints.push(Constraint::MustBeF32(
                                expr_id,
                                format!("static {} is f32", name),
                            ));
                        }
                        FloatType::F64 => {
                            self.constraints.push(Constraint::MustBeF64(
                                expr_id,
                                format!("static {} is f64", name),
                            ));
                        }
                        FloatType::Unknown => {}
                    }
                }
            }
            Item::Mod { name, items, .. } => {
                self.current_file_module_path.push(name.clone());
                for sub_item in items {
                    self.collect_item_constraints(sub_item);
                }
                self.current_file_module_path.pop();
            }
            _ => {}
        }
    }

    /// ExprIds for the "value-producing" tail of a statement list, for if/else float unification.
    /// When the last statement is a nested `if`, collects tails from both of its branches so an outer
    /// `if a { 0.0 } else { if b { 1.0 } else { x } }` still ties `0.0` to `1.0` and `x`.
    fn branch_tail_expression_ids<'ast>(&mut self, stmts: &[&'ast Statement<'ast>]) -> Vec<ExprId> {
        let Some(last) = stmts.last().copied() else {
            return Vec::new();
        };
        match last {
            Statement::Expression { expr, .. } => vec![self.get_expr_id(expr)],
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                let mut ids = self.branch_tail_expression_ids(then_block);
                if let Some(else_b) = else_block {
                    ids.extend(self.branch_tail_expression_ids(else_b));
                }
                ids
            }
            _ => Vec::new(),
        }
    }

    /// Collect constraints from a statement
    fn collect_statement_constraints<'ast>(
        &mut self,
        stmt: &Statement<'ast>,
        return_type: Option<&Type>,
    ) {
        match stmt {
            Statement::Let {
                pattern,
                value,
                type_,
                ..
            } => {
                // TDD FIX: If type annotation exists, constrain value to that type FIRST
                let explicit_type = type_.as_ref().and_then(|ty| self.extract_float_type(ty));

                // TDD: `let x: f32 = match ...` must pass f32 into the match (fn may return void).
                // Explicit non-float types (`let m: HashMap<...> = ...`) must NOT inherit the function's
                // return type as float context — that breaks `match m.get(..)` arm unification.
                let value_float_ctx: Option<&Type> = match type_.as_ref() {
                    Some(ty) if self.extract_float_type(ty).is_some() => Some(ty),
                    Some(_) => None,
                    None => return_type,
                };
                self.collect_expression_constraints(value, value_float_ctx);

                // Track variable assignment for constraint propagation
                if let crate::parser::ast::core::Pattern::Identifier(var_name) = pattern {
                    let value_id = self.get_expr_id(value);
                    self.var_assignments.insert(var_name.clone(), value_id);

                    // TDD FIX: Track explicit type annotations (let x: Type = ...)
                    if let Some(ty) = type_ {
                        self.var_types.insert(var_name.clone(), ty.clone());
                    } else if let Some(inferred_ty) = self.infer_type_from_expression(value) {
                        // TDD FIX: Infer variable type for assert_eq!(transform.x, 15.0)
                        // Enables FieldAccess to resolve struct type → field type → MustBeF32
                        self.var_types.insert(var_name.clone(), inferred_ty);
                    }

                    // TDD FIX: Constrain value expression to match explicit type annotation
                    if let Some(float_ty) = explicit_type {
                        let expr_id = self.get_expr_id(value);
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    expr_id,
                                    format!("let {} has explicit type f32", var_name),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    expr_id,
                                    format!("let {} has explicit type f64", var_name),
                                ));
                            }
                            FloatType::Unknown => {
                                // No constraint needed
                            }
                        }
                    }
                }

                // If this expression might be returned (in a function returning a float),
                // constrain it to the return type
                if let Some(ret_ty) = return_type {
                    if let Some(float_ty) = self.extract_float_type(ret_ty) {
                        let expr_id = self.get_expr_id(value);
                        let constraint = match float_ty {
                            FloatType::F32 => Constraint::MustBeF32(
                                expr_id,
                                "function return type f32".to_string(),
                            ),
                            FloatType::F64 => Constraint::MustBeF64(
                                expr_id,
                                "function return type f64".to_string(),
                            ),
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Statement::Expression { expr, .. } => {
                self.collect_expression_constraints(expr, return_type);

                // Implicit return: last expression in function body
                if let Some(ret_ty) = return_type {
                    if let Some(float_ty) = self.extract_float_type(ret_ty) {
                        let expr_id = self.get_expr_id(expr);
                        let constraint = match float_ty {
                            FloatType::F32 => {
                                Constraint::MustBeF32(expr_id, "implicit return f32".to_string())
                            }
                            FloatType::F64 => {
                                Constraint::MustBeF64(expr_id, "implicit return f64".to_string())
                            }
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                self.collect_expression_constraints(expr, return_type);

                // Return expression must match function return type
                if let Some(ret_ty) = return_type {
                    if let Some(float_ty) = self.extract_float_type(ret_ty) {
                        let expr_id = self.get_expr_id(expr);
                        let constraint = match float_ty {
                            FloatType::F32 => {
                                Constraint::MustBeF32(expr_id, "return type".to_string())
                            }
                            FloatType::F64 => {
                                Constraint::MustBeF64(expr_id, "return type".to_string())
                            }
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Statement::Return { value: None, .. } => {
                // Empty return, nothing to constrain
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                // THE WINDJAMMER WAY: if-else branches that return floats must match return type
                self.collect_expression_constraints(condition, return_type);
                for stmt in then_block {
                    self.collect_statement_constraints(stmt, return_type);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.collect_statement_constraints(stmt, return_type);
                    }
                }
                // Unify float types across then/else tails, including when else ends with nested if
                // (e.g. blend_tree: `if factor < 0.0 { 0.0 } else { if factor > 1.0 { 1.0 } else { factor } }`).
                let then_tails = self.branch_tail_expression_ids(then_block);
                if let Some(else_stmts) = else_block {
                    let else_tails = self.branch_tail_expression_ids(else_stmts);
                    for &t_id in &then_tails {
                        for &e_id in &else_tails {
                            self.constraints.push(Constraint::MustMatch(
                                t_id,
                                e_id,
                                "if/else branches must have same float type".to_string(),
                            ));
                        }
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                // TDD FIX: Traverse while loop body to find float literals in struct fields
                self.collect_expression_constraints(condition, return_type);
                for stmt in body {
                    self.collect_statement_constraints(stmt, return_type);
                }
            }
            Statement::For { iterable, body, .. } => {
                // TDD FIX: Traverse for loop body (same as While loop)
                self.collect_expression_constraints(iterable, return_type);
                for stmt in body {
                    self.collect_statement_constraints(stmt, return_type);
                }
            }
            Statement::Assignment { target, value, .. } => {
                // TDD FIX: Handle assignments (e.g., self.vy = self.vy * 0.5)
                // Traverse both target and value to collect constraints
                self.collect_expression_constraints(target, return_type);
                self.collect_expression_constraints(value, return_type);

                // THE WINDJAMMER WAY: Target and value must match types
                // For `self.vy = self.vy * 0.5`, this ensures the literal matches the field type
                let target_id = self.get_expr_id(target);
                let value_id = self.get_expr_id(value);
                self.constraints.push(Constraint::MustMatch(
                    target_id,
                    value_id,
                    "assignment target and value".to_string(),
                ));

                // TDD: Direct RHS constraint from inferred LHS type (field, index, deref, etc.).
                // Belt-and-suspenders when MustMatch/unification misses (qualified struct keys, edge cases).
                if let Some(lhs_ty) = self.infer_type_from_expression(target) {
                    if let Some(float_ty) = self.extract_float_type(&lhs_ty) {
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    value_id,
                                    "assignment RHS matches LHS float type".to_string(),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    value_id,
                                    "assignment RHS matches LHS float type".to_string(),
                                ));
                            }
                            FloatType::Unknown => {}
                        }
                    }
                }
            }
            Statement::Match { value, arms, .. } => {
                // THE WINDJAMMER WAY: Match arms must have compatible types AND match return type
                self.collect_expression_constraints(value, return_type);

                // TDD: When the function does not return f32/f64 (e.g. void), still unify arms using
                // the scrutinee (`Option<f32>`, `Result<f32, _>`, `&f32`, etc.).
                let arm_context: Option<Type> = if let Some(rt) = return_type {
                    if self.extract_float_type(rt).is_some() {
                        Some(rt.clone())
                    } else {
                        self.infer_type_from_expression(value)
                            .and_then(|ty| self.float_type_after_peeling_wrappers(ty))
                    }
                } else {
                    self.infer_type_from_expression(value)
                        .and_then(|ty| self.float_type_after_peeling_wrappers(ty))
                };
                // Never fall back to non-float return types (e.g. `-> i32`): that would pass a bogus
                // context into arm bodies and block scrutinee-only float inference for `match map.get(..)`.
                let arm_ctx_ref: Option<&Type> = arm_context
                    .as_ref()
                    .or_else(|| return_type.filter(|rt| self.extract_float_type(rt).is_some()));

                // Traverse all arms to collect constraints
                for (i, arm) in arms.iter().enumerate() {
                    self.collect_expression_constraints(arm.body, arm_ctx_ref);

                    // TDD FIX: Constrain arm to return type if function returns float
                    if let Some(ret_ty) = arm_ctx_ref {
                        if let Some(float_ty) = self.extract_float_type(ret_ty) {
                            let arm_id = self.get_expr_id(arm.body);
                            let constraint = match float_ty {
                                FloatType::F32 => Constraint::MustBeF32(
                                    arm_id,
                                    format!("match arm {} float context", i),
                                ),
                                FloatType::F64 => Constraint::MustBeF64(
                                    arm_id,
                                    format!("match arm {} float context", i),
                                ),
                                FloatType::Unknown => continue,
                            };
                            self.constraints.push(constraint);
                        }
                    }

                    if let Some(guard) = arm.guard {
                        self.collect_expression_constraints(guard, arm_ctx_ref);
                    }
                }

                // TDD FIX: All match arms must have the same type
                if arms.len() > 1 {
                    for i in 0..arms.len() - 1 {
                        let id1 = self.get_expr_id(arms[i].body);
                        let id2 = self.get_expr_id(arms[i + 1].body);
                        self.constraints.push(Constraint::MustMatch(
                            id1,
                            id2,
                            format!("match arms {} and {}", i, i + 1),
                        ));
                    }
                }
            }
            _other => {}
        }
    }
}
