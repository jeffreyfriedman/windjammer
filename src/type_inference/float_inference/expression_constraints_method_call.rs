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

        if (method == "min" || method == "max") && arguments.len() == 1 {
            let receiver_id = self.get_expr_id(object);
            let arg_id = self.get_expr_id(arguments[0].1);

            self.constraints.push(Constraint::MustMatch(
                receiver_id,
                arg_id,
                format!(".{}() argument must match receiver type", method),
            ));
        }

        let method_sig = self
            .function_signatures
            .iter()
            .filter(|(func_name, (params, _))| {
                let name_match = func_name.split("::").last() == Some(method);
                let param_match =
                    params.len() == arguments.len() + 1 || params.len() == arguments.len();
                name_match && param_match
            })
            .map(|(_, (params, ret))| (params.clone(), ret.clone()))
            .next();

        if let Some((param_types, _)) = method_sig {
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

        if let Expression::Identifier { name, .. } = object {
            if let Some(var_type) = self.var_types.get(name).cloned() {
                if method == "insert" {
                    if let Some(value_type) = self.extract_hashmap_value_type(&var_type) {
                        if let Some(float_ty) = self.extract_float_type(&value_type) {
                            if arguments.len() >= 2 {
                                let value_arg = arguments[1].1;
                                let value_id = self.get_expr_id(value_arg);
                                match float_ty {
                                    FloatType::F32 => {
                                        self.constraints.push(Constraint::MustBeF32(
                                            value_id,
                                            "HashMap<K, f32>.insert(K, f32)".to_string(),
                                        ));
                                    }
                                    FloatType::F64 => {
                                        self.constraints.push(Constraint::MustBeF64(
                                            value_id,
                                            "HashMap<K, f64>.insert(K, f64)".to_string(),
                                        ));
                                    }
                                    FloatType::Unknown => {}
                                }
                            }
                        }
                    }
                }

                if method == "push" {
                    if let Some(elem_type) = self.extract_vec_element_type(&var_type) {
                        if let Some(float_ty) = self.extract_float_type(&elem_type) {
                            if !arguments.is_empty() {
                                let value_arg = arguments[0].1;
                                let value_id = self.get_expr_id(value_arg);
                                match float_ty {
                                    FloatType::F32 => {
                                        self.constraints.push(Constraint::MustBeF32(
                                            value_id,
                                            "Vec<f32>.push(f32)".to_string(),
                                        ));
                                    }
                                    FloatType::F64 => {
                                        self.constraints.push(Constraint::MustBeF64(
                                            value_id,
                                            "Vec<f64>.push(f64)".to_string(),
                                        ));
                                    }
                                    FloatType::Unknown => {}
                                }
                            }
                        }
                    }
                }
            } else if let Some(elem_type) = self.var_element_types.get(name).cloned() {
                if method == "push" {
                    if !arguments.is_empty() {
                        let value_arg = arguments[0].1;
                        self.collect_expression_constraints(value_arg, Some(&elem_type));
                    }
                } else if method == "insert" && arguments.len() >= 2 {
                    let value_arg = arguments[1].1;
                    self.collect_expression_constraints(value_arg, Some(&elem_type));
                }
            }
        }

        for (_label, arg) in arguments {
            self.collect_expression_constraints(arg, return_type);
        }
    }
}
