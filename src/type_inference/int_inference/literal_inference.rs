impl IntInference {
    fn extract_int_type(&self, ty: &Type) -> Option<IntType> {
        self.extract_nested_int_type(ty)
    }

    /// Extract int type from nested generics: Option<Vec<int>>, Vec<Option<int>>, etc.
    fn extract_nested_int_type(&self, ty: &Type) -> Option<IntType> {
        match ty {
            Type::Int32 => Some(IntType::I32),
            Type::Int => Some(IntType::I64),
            Type::Uint => Some(IntType::U64),
            Type::Custom(name) => match name.as_str() {
                "i32" => Some(IntType::I32),
                "i64" => Some(IntType::I64),
                "u32" => Some(IntType::U32),
                "u64" => Some(IntType::U64),
                "usize" => Some(IntType::Usize),
                "isize" => Some(IntType::Isize),
                "u8" => Some(IntType::U8),
                "i8" => Some(IntType::I8),
                "u16" => Some(IntType::U16),
                "i16" => Some(IntType::I16),
                _ => None,
            },
            Type::Tuple(types) => {
                for t in types {
                    if let Some(it) = self.extract_nested_int_type(t) {
                        return Some(it);
                    }
                }
                None
            }
            Type::Vec(inner) => self.extract_nested_int_type(inner),
            Type::Array(inner, _) => self.extract_nested_int_type(inner),
            Type::Parameterized(name, args) if name == "Vec" && !args.is_empty() => {
                self.extract_nested_int_type(&args[0])
            }
            Type::Parameterized(name, args) if name == "Option" && !args.is_empty() => {
                self.extract_nested_int_type(&args[0])
            }
            Type::Parameterized(name, args) if name == "HashMap" && args.len() >= 2 => {
                self.extract_nested_int_type(&args[1])
            }
            Type::Parameterized(name, args) if name == "BTreeMap" && args.len() >= 2 => {
                self.extract_nested_int_type(&args[1])
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_nested_int_type(inner)
            }
            Type::Option(inner) => self.extract_nested_int_type(inner),
            Type::Result(ok, err) => self
                .extract_nested_int_type(ok)
                .or_else(|| self.extract_nested_int_type(err)),
            _ => None,
        }
    }

    /// Extract inner type T from Option<T> (handles Option, Parameterized, Reference)
    fn extract_option_inner_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Option(inner) => Some((**inner).clone()),
            Type::Parameterized(name, args) if name == "Option" && !args.is_empty() => {
                Some(args[0].clone())
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_option_inner_type(inner)
            }
            _ => None,
        }
    }

    /// Infer Type from an expression (for receiver type resolution in method calls)
    /// TDD: Enables HashMap<K,V>.insert and Vec<T>.push generic type propagation
    fn infer_type_from_expression<'ast>(&self, expr: &Expression<'ast>) -> Option<Type> {
        match expr {
            Expression::StructLiteral { name, .. } => Some(Type::Custom(name.clone())),
            Expression::Call { function, .. } => {
                let func_name = match function {
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier {
                            name: type_name, ..
                        } = object
                        {
                            Some(format!("{}::{}", type_name, field))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                func_name.and_then(|name| {
                    self.function_signatures
                        .get(&name)
                        .and_then(|(_, ret)| ret.clone())
                })
            }
            Expression::Identifier { name, .. } => {
                if name == "self" {
                    self.current_impl_type
                        .as_ref()
                        .map(|s| Type::Custom(s.clone()))
                } else {
                    self.var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                        .cloned()
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                let struct_name = match &object_type {
                    Type::Custom(name) => name.clone(),
                    _ => return None,
                };
                // TDD FIX: Strip generic params for struct field lookup
                // `ObjectPool<T>` → `ObjectPool`
                let base_name = if let Some(idx) = struct_name.find('<') {
                    &struct_name[..idx]
                } else {
                    &struct_name
                };
                let fields = if matches!(
                    *object,
                    Expression::Identifier { ref name, .. } if name == "self"
                ) {
                    self.current_impl_type
                        .as_deref()
                        .and_then(|ty| self.lookup_struct_fields_for_impl_type(ty))
                } else {
                    self.lookup_struct_fields(base_name)
                };
                fields.and_then(|m| m.get(field)).cloned()
            }
            Expression::Index { object, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                self.extract_vec_element_type(&object_type)
            }
            Expression::Cast { type_, .. } => Some(type_.clone()),
            Expression::Binary { left, op, .. } => {
                // TDD FIX: Binary operations return the type of their operands
                // For arithmetic (Add, Sub, Mul, Div, Mod): result type = operand type
                // For comparison (Eq, Lt, Gt, etc.): result type = bool
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod
                    | BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => {
                        // Arithmetic: result has same type as operands
                        self.infer_type_from_expression(left)
                    }
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge => {
                        // Comparison: result is bool
                        Some(Type::Bool)
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        // Logical: result is bool
                        Some(Type::Bool)
                    }
                }
            }
            _ => None,
        }
    }

    /// Extract key type K from HashMap<K,V> or BTreeMap<K,V>
    fn extract_map_key_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if matches!(base, "HashMap" | "BTreeMap" | "Map") && args.len() >= 2 {
                    Some(args[0].clone())
                } else {
                    None
                }
            }
            Type::Custom(name) if name.contains('<') => {
                let base = name.split('<').next().unwrap_or(name);
                if matches!(base, "HashMap" | "BTreeMap" | "Map") {
                    if let (Some(start), Some(end)) = (name.find('<'), name.rfind('>')) {
                        let inner = &name[start + 1..end];
                        let key = inner.split(',').next()?.trim();
                        return Some(self.parse_type_from_string(key));
                    }
                }
                None
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_map_key_type(inner)
            }
            _ => None,
        }
    }

    /// Extract value type V from HashMap<K,V> or BTreeMap<K,V>
    fn extract_map_value_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if matches!(base, "HashMap" | "BTreeMap" | "Map") && args.len() >= 2 {
                    Some(args[1].clone())
                } else {
                    None
                }
            }
            Type::Custom(name) if name.contains('<') => {
                let base = name.split('<').next().unwrap_or(name);
                if matches!(base, "HashMap" | "BTreeMap" | "Map") {
                    if let (Some(start), Some(end)) = (name.find('<'), name.rfind('>')) {
                        let inner = &name[start + 1..end];
                        let value = inner.split(',').nth(1)?.trim();
                        return Some(self.parse_type_from_string(value));
                    }
                }
                None
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_map_value_type(inner)
            }
            _ => None,
        }
    }

    /// Extract element type T from Vec<T>
    fn extract_vec_element_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some((**inner).clone()),
            Type::Parameterized(name, args) if name == "Vec" && !args.is_empty() => {
                Some(args[0].clone())
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_vec_element_type(inner)
            }
            _ => None,
        }
    }

    /// Resolve `(param_types, return_type)` for a method call from `function_signatures`,
    /// including generic substitution from the receiver type.
    fn resolve_method_signature<'ast>(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_count: usize,
        context_type: Option<&Type>,
    ) -> Option<(Vec<Type>, Option<Type>)> {
        let receiver_type = self
            .infer_type_from_expression(object)
            .or_else(|| context_type.cloned())?;
        let receiver_is_map = self.extract_map_key_type(&receiver_type).is_some();
        let (qualified, receiver_generics) =
            self.qualified_method_key_and_generics(&receiver_type, method);

        let substitute = |params: Vec<Type>| {
            let mut out: Vec<Type> = if receiver_generics.is_empty() {
                params
            } else {
                params
                    .into_iter()
                    .map(|ty| self.substitute_generic_params_typed(&ty, &receiver_generics))
                    .collect()
            };
            for param in &mut out {
                if let Type::Custom(name) = param {
                    if name.len() == 1 && name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                        match name.as_str() {
                            "V" => {
                                if let Some(v) = self.extract_map_value_type(&receiver_type) {
                                    *param = v;
                                } else if let Some(ctx) = context_type {
                                    if let Some(v) = self.extract_map_value_type(ctx) {
                                        *param = v;
                                    }
                                }
                            }
                            "T" => {
                                if let Some(t) = self.extract_vec_element_type(&receiver_type) {
                                    *param = t;
                                }
                            }
                            "K" => {
                                if let Some(k) = self.extract_map_key_type(&receiver_type) {
                                    *param = k;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            out
        };

        if let Some((params, ret)) = self.function_signatures.get(&qualified).cloned() {
            return Some((substitute(params), ret));
        }

        if receiver_is_map {
            return None;
        }

        self.function_signatures
            .iter()
            .filter(|(func_name, (params, _))| {
                func_name.split("::").last() == Some(method)
                    && (params.len() == arg_count + 1 || params.len() == arg_count)
            })
            .map(|(_, (params, ret))| (substitute(params.clone()), ret.clone()))
            .next()
    }

    fn qualified_method_key_and_generics(
        &self,
        receiver_type: &Type,
        method: &str,
    ) -> (String, Vec<Type>) {
        match receiver_type {
            Type::Parameterized(base, type_params) => {
                (format!("{}::{}", base, method), type_params.clone())
            }
            Type::Custom(n) => {
                let base = n.split('<').next().unwrap_or(n);
                let generics = if n.contains('<') {
                    if let (Some(start), Some(end)) = (n.find('<'), n.rfind('>')) {
                        let inner = &n[start + 1..end];
                        inner
                            .split(',')
                            .map(|s| self.parse_type_from_string(s.trim()))
                            .collect()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };
                (format!("{}::{}", base, method), generics)
            }
            Type::Vec(_) => (format!("Vec::{}", method), vec![]),
            _ => (String::new(), vec![]),
        }
    }

    fn method_call_returns_usize<'ast>(&self, expr: &Expression<'ast>) -> bool {
        let Expression::MethodCall {
            object,
            method,
            arguments,
            ..
        } = expr
        else {
            return false;
        };
        self.resolve_method_signature(object, method, arguments.len(), None)
            .and_then(|(_, ret)| ret)
            .and_then(|ty| self.extract_int_type(&ty))
            .is_some_and(|t| t == IntType::Usize)
    }

    fn apply_method_param_int_constraints<'ast>(
        &mut self,
        param_types: &[Type],
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        method: &str,
        return_type: Option<&Type>,
    ) {
        let param_offset = if param_types.len() == arguments.len() + 1 {
            1
        } else {
            0
        };
        for (i, (_label, arg)) in arguments.iter().enumerate() {
            if let Some(param_type) = param_types.get(i + param_offset) {
                if let Some(int_ty) = self.extract_int_type(param_type) {
                    let arg_id = self.get_expr_id(arg);
                    self.constraints.push(IntConstraint::MustBe(
                        arg_id,
                        int_ty,
                        format!("{}() parameter {}", method, i),
                    ));
                }
            }
            self.collect_expression_constraints(arg, return_type);
        }
    }
}
