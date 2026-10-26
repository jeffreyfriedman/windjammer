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

        use crate::analyzer::{SignatureRegistry, stdlib_method_traits};

        let receiver_type = self.infer_type_from_expression(object);
        if stdlib_method_traits::method_float_args_match_receiver(
            method,
            receiver_type.as_ref(),
            SignatureRegistry::stdlib(),
        ) && method_return_type.is_some()
        {
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

        // NUM-PROPAGATE: `<receiver>.min(1.0)` / `.max(0.0)` / `.clamp(..)` where the receiver's type is
        // not known yet (an untyped local, a chain): a float *literal* argument can only be accepted
        // by a float receiver, and these methods take and return the receiver's own type. Put the
        // receiver, the arguments and the result in one class so the literal follows the receiver
        // instead of defaulting (`v.min(1.0).max(0.0)` on f32 must not emit `0.0_f64`).
        let float_literal_arg = |a: &Expression<'ast>| {
            matches!(a, Expression::Literal { value: crate::parser::Literal::Float(_), .. })
        };
        if method_return_type.is_none()
            && arguments.iter().any(|(_, a)| float_literal_arg(a))
            && stdlib_method_traits::method_float_args_match_receiver(
                method,
                Some(&Type::Custom("f64".to_string())),
                SignatureRegistry::stdlib(),
            )
        {
            let receiver_id = self.get_expr_id(object);
            let call_id = self.get_expr_id(expr);
            self.constraints.push(Constraint::MustMatch(
                call_id,
                receiver_id,
                format!(".{}() preserves the receiver's float type", method),
            ));
            for (_label, arg) in arguments.iter() {
                let arg_id = self.get_expr_id(arg);
                self.constraints.push(Constraint::MustMatch(
                    receiver_id,
                    arg_id,
                    format!(".{}() argument must match receiver type", method),
                ));
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
