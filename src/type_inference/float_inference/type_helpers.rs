impl FloatInference {
    /// Used when let x = expr has no explicit type - infer from expr for assert_eq!(x.field, literal)
    /// TDD FIX: Added Binary and MethodCall fallback for len > 0.0 pattern (physics/advanced_collision.wj)
    fn infer_type_from_expression<'ast>(&self, expr: &Expression<'ast>) -> Option<Type> {
        match expr {
            Expression::StructLiteral { name, .. } => Some(Type::Custom(name.clone())),
            Expression::Binary {
                left, right, op, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                if matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                ) {
                    let left_ty = self.infer_type_from_expression(left)?;
                    let right_ty = self.infer_type_from_expression(right)?;
                    if left_ty == right_ty {
                        return Some(left_ty);
                    }
                }
                None
            }
            // TDD: `let x = (n as f32) / (m as f32)` must record x as f32 so `x < 0.3` constrains the literal.
            Expression::Cast { type_, .. } => {
                self.extract_float_type(type_).and_then(|ft| match ft {
                    FloatType::F32 => Some(Type::Custom("f32".to_string())),
                    FloatType::F64 => Some(Type::Custom("f64".to_string())),
                    FloatType::Unknown => None,
                })
            }
            Expression::Call { function, .. } => {
                // Parser desugars `receiver.method(args)` as Call(FieldAccess(receiver, method), args).
                // Map shared-get must infer like MethodCall so `match m.get(..)` gets arm float context.
                if let Expression::FieldAccess { object, field, .. } = function {
                    if let Some(object_type) = self.infer_type_from_expression(object) {
                        let type_name = match &object_type {
                            Type::Custom(name) => name.clone(),
                            Type::Parameterized(name, _) => name.clone(),
                            _ => String::new(),
                        };
                        if !type_name.is_empty() {
                            let full_name = format!("{}::{}", type_name, field);
                            if let Some((_, Some(ret))) = self.function_signatures.get(&full_name) {
                                if crate::codegen::rust::stdlib_method_traits::type_is_option_shared_ref(
                                    ret,
                                ) || crate::codegen::rust::stdlib_method_traits::type_is_option_mut_ref(
                                    ret,
                                ) {
                                    if let Some(value_ty) =
                                        self.extract_map_value_type(&object_type)
                                    {
                                        return Some(Type::Option(Box::new(value_ty)));
                                    }
                                }
                            }
                        }
                    }
                }
                // Type::new() or Type::method() - get return type from function signature
                let func_name = match function {
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
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    _ => None,
                };
                func_name.and_then(|name| {
                    self.function_signatures
                        .get(&name)
                        .and_then(|(_, ret)| ret.clone())
                })
            }
            Expression::MethodCall { object, method, .. } => {
                // object.method() - need object's type to find method signature
                let object_type = self.infer_type_from_expression(object)?;
                let type_name = match &object_type {
                    Type::Custom(name) => name.clone(),
                    Type::Parameterized(name, _) => name.clone(),
                    _ => return None,
                };
                let full_name = format!("{}::{}", type_name, method);
                // Map shared-get → Option<V> (match arms need float context on value type)
                if let Some((_, Some(ret))) = self.function_signatures.get(&full_name) {
                    if crate::codegen::rust::stdlib_method_traits::type_is_option_shared_ref(ret)
                        || crate::codegen::rust::stdlib_method_traits::type_is_option_mut_ref(ret)
                    {
                        if let Some(value_ty) = self.extract_map_value_type(&object_type) {
                            return Some(Type::Option(Box::new(value_ty)));
                        }
                    }
                }
                if let Some((_, ret)) = self.function_signatures.get(&full_name) {
                    return ret.clone();
                }
                if crate::analyzer::stdlib_method_traits::method_preserves_float_receiver(
                    method,
                    Some(&object_type),
                    crate::analyzer::SignatureRegistry::stdlib(),
                ) {
                    return Some(object_type);
                }
                None
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
                let base_name = if let Some(idx) = struct_name.find('<') {
                    &struct_name[..idx]
                } else {
                    struct_name.as_str()
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
            Expression::Unary { op, operand, .. } => {
                use crate::parser::ast::operators::UnaryOp;
                if matches!(op, UnaryOp::Deref) {
                    self.infer_type_from_expression(operand)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Extract FloatType from a Type
    fn extract_float_type(&self, ty: &Type) -> Option<FloatType> {
        match ty {
            Type::Float => Some(FloatType::F64),
            Type::Custom(name) if name == "f32" => Some(FloatType::F32),
            Type::Custom(name) if name == "f64" => Some(FloatType::F64),
            Type::Custom(name) => {
                // Resolve type aliases: e.g., Quat = (f32, f32, f32, f32)
                if let Some(resolved) = self.type_aliases.get(name.as_str()) {
                    self.extract_float_type(resolved)
                } else {
                    None
                }
            }
            Type::Tuple(types) => {
                for t in types {
                    if let Some(float_ty) = self.extract_float_type(t) {
                        return Some(float_ty);
                    }
                }
                None
            }
            Type::Vec(inner) | Type::Array(inner, _) => self.extract_float_type(inner),
            Type::Option(inner) => self.extract_float_type(inner),
            Type::Result(ok_type, _) => self.extract_float_type(ok_type),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_float_type(inner)
            }
            Type::Parameterized(name, type_args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if (base == "Vec" || base == "Option" || base == "Result") && !type_args.is_empty()
                {
                    self.extract_float_type(&type_args[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Value type `V` from `HashMap<K, V>` or Windjammer `Map<K, V>`.
    fn extract_map_value_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, type_args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if matches!(base, "HashMap" | "Map" | "BTreeMap") && type_args.len() >= 2 {
                    Some(type_args[1].clone())
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

    /// If `receiver` is map-like and `V` is a scalar float, return that float type.
    fn map_receiver_value_float_type<'ast>(
        &self,
        receiver: &Expression<'ast>,
    ) -> Option<FloatType> {
        let object_type = self.infer_type_from_expression(receiver)?;
        let value_ty = self.extract_map_value_type(&object_type)?;
        self.extract_float_type(&value_ty)
    }

    /// TDD FIX: Extract value type V from HashMap<K, V> (alias for map-like containers)
    fn extract_hashmap_value_type(&self, ty: &Type) -> Option<Type> {
        self.extract_map_value_type(ty)
    }

    /// TDD FIX: Extract element type T from Vec<T>
    fn extract_vec_element_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some((**inner).clone()),
            Type::Parameterized(name, type_args) if name == "Vec" => {
                // Vec<T> has 1 type argument
                if !type_args.is_empty() {
                    Some(type_args[0].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn parse_type_from_string(&self, s: &str) -> Type {
        match s {
            "i32" => Type::Int32,
            "i64" => Type::Int,
            "u64" => Type::Custom("u64".to_string()),
            "u32" => Type::Custom("u32".to_string()),
            "usize" => Type::Custom("usize".to_string()),
            "u8" => Type::Custom("u8".to_string()),
            "i8" => Type::Custom("i8".to_string()),
            "u16" => Type::Custom("u16".to_string()),
            "i16" => Type::Custom("i16".to_string()),
            "f32" => Type::Float,
            "f64" => Type::Float,
            "bool" => Type::Bool,
            "string" => Type::String,
            _ => Type::Custom(s.to_string()),
        }
    }

    fn substitute_generic_params_typed(&self, ty: &Type, generics: &[Type]) -> Type {
        match ty {
            Type::Custom(name) if name.len() == 1 => {
                let ch = name.chars().next().unwrap();
                if ch.is_ascii_uppercase() {
                    let idx = match ch {
                        'K' => 0,
                        'V' => 1,
                        'T' => 0,
                        'U' => 1,
                        'E' => 1,
                        _ => return ty.clone(),
                    };
                    if let Some(concrete) = generics.get(idx) {
                        return concrete.clone();
                    }
                }
                ty.clone()
            }
            Type::Option(inner) => Type::Option(Box::new(
                self.substitute_generic_params_typed(inner, generics),
            )),
            Type::Result(ok, err) => Type::Result(
                Box::new(self.substitute_generic_params_typed(ok, generics)),
                Box::new(self.substitute_generic_params_typed(err, generics)),
            ),
            Type::Vec(inner) => Type::Vec(Box::new(
                self.substitute_generic_params_typed(inner, generics),
            )),
            _ => ty.clone(),
        }
    }

    /// Resolve method param types with generic substitution from the receiver type.
    fn resolve_method_param_types<'ast>(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_count: usize,
        context_type: Option<&Type>,
    ) -> Option<Vec<Type>> {
        let receiver_type = self
            .infer_type_from_expression(object)
            .or_else(|| context_type.cloned())?;
        // Bare `HashMap` must still block suffix fallback (Vec::insert).
        let receiver_is_map = self.extract_map_key_type(&receiver_type).is_some()
            || match &receiver_type {
                Type::Custom(n) => {
                    let base = n.split('<').next().unwrap_or(n);
                    matches!(base, "HashMap" | "BTreeMap" | "Map" | "IndexMap")
                }
                Type::Parameterized(base, _) => {
                    let b = crate::type_inference::generic_type_base_name(base);
                    matches!(b, "HashMap" | "BTreeMap" | "Map" | "IndexMap")
                }
                _ => false,
            };

        let (qualified, receiver_generics) = match &receiver_type {
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
            _ => return None,
        };

        let substitute = |params: Vec<Type>| {
            let mut out: Vec<Type> = if receiver_generics.is_empty() {
                params
            } else {
                params
                    .into_iter()
                    .map(|ty| self.substitute_generic_params_typed(&ty, &receiver_generics))
                    .collect()
            };
            // Fill remaining single-letter generics from receiver shape (HashMap<K,V>, Vec<T>).
            for param in &mut out {
                if let Type::Custom(name) = param {
                    if name.len() == 1
                        && name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    {
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

        if let Some((params, _)) = self.function_signatures.get(&qualified).cloned() {
            return Some(substitute(params));
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
            .map(|(_, (params, _))| substitute(params.clone()))
            .next()
    }

    fn extract_map_key_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, args)
                if (name == "HashMap" || name == "BTreeMap") && !args.is_empty() =>
            {
                Some(args[0].clone())
            }
            Type::Custom(name) if name.starts_with("HashMap<") || name.starts_with("BTreeMap<") => {
                if let (Some(start), Some(end)) = (name.find('<'), name.rfind('>')) {
                    let inner = &name[start + 1..end];
                    let first = inner.split(',').next()?.trim();
                    Some(self.parse_type_from_string(first))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Peel `Option` / `Result` / references until we reach a concrete type, then keep it if it is f32/f64.
    fn float_type_after_peeling_wrappers(&self, mut ty: Type) -> Option<Type> {
        loop {
            match ty {
                Type::Option(inner) => ty = (*inner).clone(),
                Type::Result(ok, _) => ty = (*ok).clone(),
                Type::Reference(inner) | Type::MutableReference(inner) => ty = (*inner).clone(),
                Type::Parameterized(name, ref args) if name == "Option" && args.len() == 1 => {
                    ty = args[0].clone();
                }
                Type::Parameterized(name, ref args) if name == "Result" && !args.is_empty() => {
                    ty = args[0].clone();
                }
                _ => break,
            }
        }
        if self.extract_float_type(&ty).is_some() {
            Some(ty)
        } else {
            None
        }
    }

    /// Get unique ID for an expression (based on source location)
    /// Get unique ID for expression with location-based caching
    /// THE WINDJAMMER WAY: Cache by location to ensure same expression = same ID
    /// This fixes the problem where same expression got multiple IDs during traversal
    fn get_expr_id<'ast>(&mut self, expr: &Expression<'ast>) -> ExprId {
        let location = expr.location();
        let (line, col) = if let Some(loc) = location {
            (loc.line, loc.column)
        } else {
            (0, 0)
        };

        // TDD FIX: Use file-aware cache key to prevent cross-file collisions
        let cache_key = (self.current_file_id, line, col);

        // Check cache first - if we've seen this location before, return same ID
        if line > 0 {
            // Only cache expressions with valid locations
            if let Some(&cached_id) = self.expr_id_cache.get(&cache_key) {
                return cached_id;
            }
        }

        // Generate new sequential ID (globally unique across all files)
        let seq_id = self.next_seq_id;
        self.next_seq_id += 1;

        let expr_id = ExprId {
            seq_id,
            file_id: self.current_file_id,
            line,
            col,
        };

        // Cache it for future lookups
        if line > 0 {
            self.expr_id_cache.insert(cache_key, expr_id);
        }

        expr_id
    }

    /// Determine the return type of a method call
    /// Returns Some(FloatType) if the method is known to return f32/f64, None otherwise
    fn determine_method_return_type(&self, object: &Expression, method: &str) -> Option<FloatType> {
        use crate::analyzer::{stdlib_method_traits, SignatureRegistry};

        let registry = SignatureRegistry::stdlib();

        let float_from_receiver = |receiver_type: &Type| -> Option<FloatType> {
            if stdlib_method_traits::method_preserves_float_receiver(
                method,
                Some(receiver_type),
                registry,
            ) {
                return self.extract_float_type(receiver_type);
            }
            None
        };

        // Check if this is a method call on an identifier
        if let Expression::Identifier { name, .. } = object {
            if let Some(var_type) = self.var_types.get(name) {
                if let Some(ft) = float_from_receiver(var_type) {
                    return Some(ft);
                }
            }
        }

        // Method on a field (e.g. self.vy.sqrt(), pos.x.acos())
        if let Expression::FieldAccess { .. } = object {
            if let Some(ty) = self.infer_type_from_expression(object) {
                if let Some(ft) = float_from_receiver(&ty) {
                    return Some(ft);
                }
            }
        }

        // MethodCall on MethodCall (chaining): guard — only float receivers (not Slider::max).
        if let Expression::MethodCall {
            object: inner_obj,
            method: inner_method,
            ..
        } = object
        {
            if let Some(object_type) = self.infer_type_from_expression(object) {
                if let Some(ft) = float_from_receiver(&object_type) {
                    return Some(ft);
                }
                return None;
            }
            // Chained math without type info: infer inner float return, then check outer method.
            if let Some(inner_ft) = self.determine_method_return_type(inner_obj, inner_method) {
                let ty = match inner_ft {
                    FloatType::F32 => Type::Custom("f32".to_string()),
                    FloatType::F64 => Type::Custom("f64".to_string()),
                    FloatType::Unknown => return None,
                };
                return float_from_receiver(&ty);
            }
        }

        // MethodCall on Binary (e.g., (x*x + y*y).sqrt())
        if let Expression::Binary { .. } = object {
            if let Some(object_type) = self.infer_type_from_expression(object) {
                return float_from_receiver(&object_type);
            }
        }

        None
    }
}
