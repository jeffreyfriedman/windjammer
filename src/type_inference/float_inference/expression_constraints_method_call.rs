impl FloatInference {
    fn collect_expression_constraints_method_call<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        object: &'ast Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        return_type: Option<&Type>,
    ) {
        self.collect_expression_constraints(object, return_type);

        let method_return_type = self.determine_method_return_type(object, method);

        if let Some(float_ty) = method_return_type {
            let method_call_id = self.get_expr_id(expr);
            match float_ty {
                FloatType::F32 => {
                    self.constraints.push(Constraint::MustBeF32(
                        method_call_id,
                        format!("method {} returns f32", method),
                    ));
                }
                FloatType::F64 => {
                    self.constraints.push(Constraint::MustBeF64(
                        method_call_id,
                        format!("method {} returns f64", method),
                    ));
                }
                FloatType::Unknown => {}
            }
        }

        const SELF_ARG_METHODS: &[&str] = &[
            "min", "max", "clamp", "copysign", "atan2", "hypot", "powf",
        ];
        // Only add float constraints when the receiver is actually a float type.
        // Struct builder methods (e.g. Slider::max) share names with numeric methods
        // but must not trigger float inference constraints.
        if SELF_ARG_METHODS.contains(&method) && method_return_type.is_some() {
            let receiver_id = self.get_expr_id(object);
            for (_label, arg) in arguments.iter() {
                let arg_id = self.get_expr_id(arg);
                self.constraints.push(Constraint::MustMatch(
                    receiver_id,
                    arg_id,
                    format!(".{}() argument must match receiver type", method),
                ));
            }
            if let Some(ref float_ty) = method_return_type {
                for (_label, arg) in arguments.iter() {
                    let arg_id = self.get_expr_id(arg);
                    match float_ty {
                        FloatType::F32 => {
                            self.constraints.push(Constraint::MustBeF32(
                                arg_id,
                                format!(".{}() arg must be f32 (matches receiver)", method),
                            ));
                        }
                        FloatType::F64 => {
                            self.constraints.push(Constraint::MustBeF64(
                                arg_id,
                                format!(".{}() arg must be f64 (matches receiver)", method),
                            ));
                        }
                        FloatType::Unknown => {}
                    }
                }
            }
        }

        // Signature-driven param constraints (includes generic substitution for HashMap/Vec).
        if let Some(param_types) =
            self.resolve_method_param_types(object, method, arguments.len(), return_type)
        {
            let param_offset = if param_types.len() == arguments.len() + 1 {
                1
            } else {
                0
            };
            for (i, (_label, arg)) in arguments.iter().enumerate() {
                if let Some(param_type) = param_types.get(i + param_offset) {
                    if let Some(float_ty) = self.extract_float_type(param_type) {
                        let arg_id = self.get_expr_id(arg);
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    arg_id,
                                    format!("{}() parameter {}", method, i),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    arg_id,
                                    format!("{}() parameter {}", method, i),
                                ));
                            }
                            FloatType::Unknown => {}
                        }
                    }
                }
            }
        }

        for (_label, arg) in arguments {
            self.collect_expression_constraints(arg, return_type);
        }
    }
}
