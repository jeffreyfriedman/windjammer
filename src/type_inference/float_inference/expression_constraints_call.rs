impl FloatInference {
    fn collect_expression_constraints_call<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        function: &'ast Expression<'ast>,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        return_type: Option<&Type>,
    ) {
        if let Expression::FieldAccess { object, field, .. } = function {
            if field == "get" {
                if let Some(float_ty) = self.map_receiver_value_float_type(object) {
                    let call_id = self.get_expr_id(expr);
                    match float_ty {
                        FloatType::F32 => {
                            self.constraints.push(Constraint::MustBeF32(
                                call_id,
                                "map get optional value is f32".to_string(),
                            ));
                        }
                        FloatType::F64 => {
                            self.constraints.push(Constraint::MustBeF64(
                                call_id,
                                "map get optional value is f64".to_string(),
                            ));
                        }
                        FloatType::Unknown => {}
                    }
                }
            }
        }

        if let Expression::FieldAccess { object, field, .. } = function {
            if let Some(float_ty) = self.determine_method_return_type(object, field.as_str()) {
                let call_id = self.get_expr_id(expr);
                match float_ty {
                    FloatType::F32 => {
                        self.constraints.push(Constraint::MustBeF32(
                            call_id,
                            format!("call {} returns f32", field),
                        ));
                    }
                    FloatType::F64 => {
                        self.constraints.push(Constraint::MustBeF64(
                            call_id,
                            format!("call {} returns f64", field),
                        ));
                    }
                    FloatType::Unknown => {}
                }
            }
        }

        self.collect_expression_constraints(function, return_type);

        let lookup_keys = Self::call_signature_lookup_keys(function);
        let func_sig = lookup_keys
            .iter()
            .find_map(|k| self.function_signatures.get(k).cloned());
        let func_name = lookup_keys
            .iter()
            .find(|k| self.function_signatures.contains_key(*k))
            .cloned()
            .or_else(|| lookup_keys.first().cloned());

        if let Some(ref name) = func_name {
            if (name == "assert_eq" || name == "assert_ne") && arguments.len() >= 2 {
                for (_label, arg) in arguments.iter() {
                    self.collect_expression_constraints(arg, return_type);
                }
                let first_id = self.get_expr_id(arguments[0].1);
                let second_id = self.get_expr_id(arguments[1].1);
                self.constraints.push(Constraint::MustMatch(
                    first_id,
                    second_id,
                    format!("{} requires both arguments to have same type", name),
                ));
                return;
            }
        }

        if let Some((param_types, _)) = func_sig {
            let label = func_name.unwrap_or_else(|| "function".to_string());
            for (i, (_label, arg)) in arguments.iter().enumerate() {
                self.collect_expression_constraints(arg, return_type);

                if let Some(param_type) = param_types.get(i) {
                    if let Some(float_ty) = self.extract_float_type(param_type) {
                        let arg_id = self.get_expr_id(arg);
                        let constraint = match float_ty {
                            FloatType::F32 => {
                                Constraint::MustBeF32(arg_id, format!("parameter {} of {}", i, label))
                            }
                            FloatType::F64 => {
                                Constraint::MustBeF64(arg_id, format!("parameter {} of {}", i, label))
                            }
                            FloatType::Unknown => continue,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
        } else {
            for (_label, arg) in arguments {
                self.collect_expression_constraints(arg, return_type);
            }
        }
    }
}
