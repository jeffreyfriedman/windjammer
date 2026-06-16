impl IntInference {
    fn solve_constraints(&mut self) {
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
                                && int_ty != IntType::Unknown =>
                            {
                                // TDD FIX: Prefer specific types (Usize, U32) over default (I32) from literals
                                if other == IntType::I32 && int_ty != IntType::I32 {
                                    // Upgrade from default I32 to specific required type
                                    self.inferred_types.insert(expr_id, int_ty);
                                    changed = true;
                                } else if int_ty == IntType::I32 && other != IntType::I32 {
                                    // Wrong generic/default I32 constraint vs specific inferred type — keep specific
                                    changed = false;
                                } else if !is_safe_implicit_cast(other, int_ty) {
                                    // Real conflict
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
                            }
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
                                if a != b && a != IntType::Unknown && b != IntType::Unknown =>
                            {
                                // TDD FIX: Prefer specific types (Usize, U32) over default (I32)
                                let preferred = if a == IntType::I32 && b != IntType::I32 {
                                    b // Prefer non-default
                                } else if b == IntType::I32 && a != IntType::I32 {
                                    a // Prefer non-default
                                } else if is_safe_implicit_cast(a, b) || is_safe_implicit_cast(b, a) {
                                    // Safe cast - keep current
                                    continue;
                                } else {
                                    // Real conflict
                                    let file_path = self
                                        .id_to_file_name
                                        .get(&id1.file_id)
                                        .map(|s| s.as_str())
                                        .unwrap_or("?");
                                    self.errors.push(format!(
                                        "{}:{}:{}: Type mismatch {}: {:?} vs {:?} ({})",
                                        file_path, id1.line, id1.col, reason, a, b, reason
                                    ));
                                    continue;
                                };
                                // Update both to preferred type
                                self.inferred_types.insert(id1, preferred);
                                self.inferred_types.insert(id2, preferred);
                                changed = true;
                            }
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
