impl IntInference {
    fn solve_constraints(&mut self) {
        // First pass: Detect conflicts in MustBe constraints BEFORE solving
        let mut expr_must_be: HashMap<ExprId, Vec<(IntType, String)>> = HashMap::new();
        for constraint in &self.constraints {
            if let IntConstraint::MustBe(expr_id, int_ty, reason) = constraint {
                expr_must_be
                    .entry(*expr_id)
                    .or_insert_with(Vec::new)
                    .push((*int_ty, reason.clone()));
            }
        }

        // Check for conflicting MustBe constraints on the same expression
        for (expr_id, constraints) in &expr_must_be {
            let mut concrete_types: Vec<(IntType, &String)> = constraints
                .iter()
                .filter_map(|(ty, reason)| {
                    if *ty != IntType::Unknown {
                        Some((*ty, reason))
                    } else {
                        None
                    }
                })
                .collect();

            // If multiple different concrete types, report first mismatch
            if concrete_types.len() > 1 {
                concrete_types.sort_by_key(|(ty, _)| format!("{:?}", ty));
                concrete_types.dedup_by_key(|(ty, _)| *ty);
                if concrete_types.len() > 1 {
                    // Real conflict - different concrete types required
                    let (first_ty, first_reason) = concrete_types[0];
                    let (second_ty, second_reason) = concrete_types[1];
                    let file_path = self
                        .id_to_file_name
                        .get(&expr_id.file_id)
                        .map(|s| s.as_str())
                        .unwrap_or("?");
                    self.errors.push(format!(
                        "{}:{}:{}: Type conflict: must be {:?} ({}) but also {:?} ({})",
                        file_path,
                        expr_id.line,
                        expr_id.col,
                        first_ty,
                        first_reason,
                        second_ty,
                        second_reason
                    ));
                    continue; // Skip this expr_id - can't resolve
                }
            }

            // No conflict - use the first concrete type (they're all the same)
            if let Some((ty, _)) = concrete_types.first() {
                self.inferred_types.insert(*expr_id, *ty);
            }
        }

        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            for constraint in self.constraints.clone() {
                match constraint {
                    IntConstraint::MustBe(expr_id, int_ty, reason) => {
                        let current = self.inferred_types.get(&expr_id).copied();
                        match current {
                            Some(other)
                                if other != int_ty && other != IntType::Unknown
                                && int_ty != IntType::Unknown
                                    // TDD FIX: Only emit error if NOT a safe implicit cast
                                    // DON'T modify inferred types - let codegen insert casts
                                    && !is_safe_implicit_cast(other, int_ty) =>
                            {
                                let file_path = self
                                    .id_to_file_name
                                    .get(&expr_id.file_id)
                                    .map(|s| s.as_str())
                                    .unwrap_or("?");
                                self.errors.push(format!(
                                    "{}:{}:{}: Type conflict: must be {:?} ({}) but was {:?}",
                                    file_path, expr_id.line, expr_id.col, int_ty, reason, other
                                ));
                            }
                            // else: Safe cast - silently allow it
                            Some(IntType::Unknown) | None => {
                                let to_insert = if int_ty == IntType::Unknown {
                                    IntType::I32
                                } else {
                                    int_ty
                                };
                                self.inferred_types.insert(expr_id, to_insert);
                                changed = true;
                            }
                            _ => {}
                        }
                    }
                    IntConstraint::MustMatch(id1, id2, reason) => {
                        let t1 = self.inferred_types.get(&id1).copied();
                        let t2 = self.inferred_types.get(&id2).copied();

                        match (t1, t2) {
                            (Some(a), Some(b))
                                if a != b && a != IntType::Unknown && b != IntType::Unknown
                                // TDD FIX: Only emit error if NOT a safe implicit cast in either direction
                                // DON'T modify inferred types - let codegen insert casts
                                && !is_safe_implicit_cast(a, b) && !is_safe_implicit_cast(b, a) =>
                            {
                                let file_path = self
                                    .id_to_file_name
                                    .get(&id1.file_id)
                                    .map(|s| s.as_str())
                                    .unwrap_or("?");
                                self.errors.push(format!(
                                    "{}:{}:{}: Type mismatch {}: {:?} vs {:?} ({})",
                                    file_path, id1.line, id1.col, reason, a, b, reason
                                ));
                            }
                            // else: Safe cast in at least one direction - silently allow it
                            (Some(concrete), None | Some(IntType::Unknown)) => {
                                let to_use = if concrete == IntType::Unknown {
                                    IntType::I32
                                } else {
                                    concrete
                                };
                                self.inferred_types.insert(id2, to_use);
                                changed = true;
                            }
                            (None | Some(IntType::Unknown), Some(concrete)) => {
                                let to_use = if concrete == IntType::Unknown {
                                    IntType::I32
                                } else {
                                    concrete
                                };
                                self.inferred_types.insert(id1, to_use);
                                changed = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
