impl FloatInference {
    /// Solve constraints using unification
    /// THE WINDJAMMER WAY: Process explicit type constraints (MustBeF32/MustBeF64) before
    /// MustMatch to prevent propagation from unconstrained literals (default f64) from
    /// overwriting parameter types (e.g., dt: f32). Fixes multi-file float type conflicts.
    fn solve_constraints(&mut self) {
        // Pass 0: Establish explicit types first (params, const, struct fields)
        // This prevents MustMatch from propagating f64 from untyped literals to typed params
        for constraint in &self.constraints {
            match constraint {
                Constraint::MustBeF32(expr_id, _) => {
                    if !self.inferred_types.contains_key(expr_id) {
                        self.inferred_types.insert(*expr_id, FloatType::F32);
                    }
                }
                Constraint::MustBeF64(expr_id, _) => {
                    if !self.inferred_types.contains_key(expr_id) {
                        self.inferred_types.insert(*expr_id, FloatType::F64);
                    }
                }
                Constraint::MustMatch(_, _, _) => {}
            }
        }

        // Pass 1+: Unification with conflict detection
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            for constraint in self.constraints.clone() {
                match constraint {
                    Constraint::MustBeF32(expr_id, reason) => {
                        // THE WINDJAMMER WAY: Always insert if missing, check conflicts if present
                        let current = self.inferred_types.get(&expr_id).copied();
                        match current {
                            Some(FloatType::F64) => {
                                if std::env::var("WJ_DEBUG_FLOAT_CONFLICT").is_ok() {
                                    let source_text = self
                                        .debug_source
                                        .as_ref()
                                        .and_then(|s| s.lines().nth(expr_id.line.saturating_sub(1)))
                                        .unwrap_or("")
                                        .trim();
                                    eprintln!(
                                        "DEBUG: Type conflict at expr seq_id={} ({}:{})",
                                        expr_id.seq_id, expr_id.line, expr_id.col
                                    );
                                    eprintln!("  Current type: F64");
                                    eprintln!("  Required type: F32 ({})", reason);
                                    eprintln!("  Source line {}: {}", expr_id.line, source_text);
                                }
                                self.errors.push(format!(
                                    "Type conflict at seq_id={}, {}:{}: must be f32 ({}) but was inferred as f64",
                                    expr_id.seq_id, expr_id.line, expr_id.col, reason
                                ));
                            }
                            Some(FloatType::F32) => {
                                // Already F32, no change needed
                            }
                            Some(FloatType::Unknown) => {
                                // Unknown -> F32
                                self.inferred_types.insert(expr_id, FloatType::F32);
                                changed = true;
                            }
                            None => {
                                // Not yet inferred, insert f32
                                self.inferred_types.insert(expr_id, FloatType::F32);
                                changed = true;
                            }
                        }
                    }
                    Constraint::MustBeF64(expr_id, reason) => {
                        // THE WINDJAMMER WAY: Always insert if missing, check conflicts if present
                        let current = self.inferred_types.get(&expr_id).copied();
                        match current {
                            Some(FloatType::F32) => {
                                if std::env::var("WJ_DEBUG_FLOAT_CONFLICT").is_ok() {
                                    let source_text = self
                                        .debug_source
                                        .as_ref()
                                        .and_then(|s| s.lines().nth(expr_id.line.saturating_sub(1)))
                                        .unwrap_or("")
                                        .trim();
                                    eprintln!(
                                        "DEBUG: Type conflict at expr seq_id={} ({}:{})",
                                        expr_id.seq_id, expr_id.line, expr_id.col
                                    );
                                    eprintln!("  Current type: F32");
                                    eprintln!("  Required type: F64 ({})", reason);
                                    eprintln!("  Source line {}: {}", expr_id.line, source_text);
                                }
                                self.errors.push(format!(
                                    "Type conflict at seq_id={}, {}:{}: must be f64 ({}) but was inferred as f32",
                                    expr_id.seq_id, expr_id.line, expr_id.col, reason
                                ));
                            }
                            Some(FloatType::F64) => {
                                // Already F64, no change needed
                            }
                            Some(FloatType::Unknown) => {
                                // Unknown -> F64
                                self.inferred_types.insert(expr_id, FloatType::F64);
                                changed = true;
                            }
                            None => {
                                // Not yet inferred, insert f64
                                self.inferred_types.insert(expr_id, FloatType::F64);
                                changed = true;
                            }
                        }
                    }
                    Constraint::MustMatch(id1, id2, reason) => {
                        let ty1 = self.inferred_types.get(&id1).copied();
                        let ty2 = self.inferred_types.get(&id2).copied();

                        match (ty1, ty2) {
                            // Conflict: f32 vs f64
                            (Some(FloatType::F32), Some(FloatType::F64))
                            | (Some(FloatType::F64), Some(FloatType::F32)) => {
                                self.errors.push(format!(
                                    "Type mismatch at {:?} and {:?}: {} requires same float type",
                                    id1, id2, reason
                                ));
                            }
                            // Propagate f32 to unknown or untyped
                            (Some(FloatType::F32), Some(FloatType::Unknown))
                            | (Some(FloatType::F32), None) => {
                                self.inferred_types.insert(id2, FloatType::F32);
                                changed = true;
                            }
                            (Some(FloatType::Unknown), Some(FloatType::F32))
                            | (None, Some(FloatType::F32)) => {
                                self.inferred_types.insert(id1, FloatType::F32);
                                changed = true;
                            }
                            // Propagate f64 to unknown or untyped
                            (Some(FloatType::F64), Some(FloatType::Unknown))
                            | (Some(FloatType::F64), None) => {
                                self.inferred_types.insert(id2, FloatType::F64);
                                changed = true;
                            }
                            (Some(FloatType::Unknown), Some(FloatType::F64))
                            | (None, Some(FloatType::F64)) => {
                                self.inferred_types.insert(id1, FloatType::F64);
                                changed = true;
                            }
                            // Both same concrete type - no change
                            (Some(FloatType::F32), Some(FloatType::F32))
                            | (Some(FloatType::F64), Some(FloatType::F64)) => {}
                            // Both unknown or untyped - wait for more constraints
                            _ => {}
                        }
                    }
                }
            }
        }

        // THE WINDJAMMER WAY: If no new changes occurred, we've converged successfully
        // Only error if we're still changing after max iterations (true infinite loop)
        if iterations >= MAX_ITERATIONS && changed {
            self.errors.push(format!(
                "Type inference did not converge after {} iterations",
                MAX_ITERATIONS
            ));
        }
    }

    /// Recursively constrain float literals in nested expressions
    /// Used for binary ops like: x * y * 0.5 (all must match)
    fn constrain_nested_floats<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        return_type: Option<&Type>,
    ) {
        match expr {
            Expression::Binary {
                left, right, op, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        let left_id = self.get_expr_id(left);
                        let right_id = self.get_expr_id(right);
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            right_id,
                            "nested binary operation".to_string(),
                        ));

                        // Recurse deeper
                        self.constrain_nested_floats(left, return_type);
                        self.constrain_nested_floats(right, return_type);
                    }
                    _ => {}
                }
            }
            Expression::Literal { .. } => {
                // Base case: literal found
            }
            Expression::Cast {
                expr: inner, type_, ..
            } => {
                // Cast expression provides explicit type hint
                if let Some(float_ty) = self.extract_float_type(type_) {
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
                self.constrain_nested_floats(inner, return_type);
            }
            _ => {}
        }
    }
    fn constrain_expr_to_type<'ast>(&mut self, expr: &Expression<'ast>, target_type: &Type) {
        if let Expression::Identifier { name, .. } = expr {
            // Variable being returned - store its element type if it's a collection
            match target_type {
                Type::Vec(inner) => {
                    // Vec<T>
                    self.var_element_types
                        .insert(name.clone(), (**inner).clone());
                }
                Type::Parameterized(type_name, parameters) => {
                    // Vec<T>, HashMap<K,V>, Option<T>, etc.
                    let base = crate::type_inference::generic_type_base_name(type_name);
                    if base == "Vec" && parameters.len() == 1 {
                        self.var_element_types
                            .insert(name.clone(), parameters[0].clone());
                    } else if matches!(base, "HashMap" | "Map" | "BTreeMap")
                        && parameters.len() == 2
                    {
                        // Store the value type (second parameter)
                        self.var_element_types
                            .insert(name.clone(), parameters[1].clone());
                    }
                }
                _ => {}
            }
        }
    }
}
